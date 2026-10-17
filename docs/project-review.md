# Revisión del proyecto — 17 de octubre de 2026

## Fallos reproducidos y corregidos

- **Declaraciones duplicadas de alfabeto.** Dos líneas `alphabet` con símbolos
  distintos se fusionaban sin diagnóstico. Ahora la segunda produce `SEM002`:
  `duplicate alphabet declaration`. Cada autómata tiene una única declaración
  opcional de alfabeto.
- **Pánicos al informar errores.** Con stderr cerrado, errores de uso, lectura,
  sintaxis o entrada terminaban con código 101 por el uso de `eprintln!`.
  Ahora la escritura de diagnósticos es falible y se conserva el código de
  salida correspondiente. Hay una regresión de proceso para cada ruta.
- **API pública y símbolos desconocidos.** `accepts` podía aceptar un símbolo
  fuera de `alphabet` si un consumidor construía un modelo con una transición
  inconsistente. Ahora se rechazan las palabras que contienen esos símbolos.
  Los archivos cargados con `parse` ya pasaban por la validación semántica.
- **Prueba de documentación incompatible con Windows.** La primera ejecución
  de CI descubrió que extraer el ejemplo del README buscando `LF` literalmente
  fallaba con los saltos `CRLF` de Windows. La prueba ahora extrae por líneas y
  comprueba ambas convenciones de salto de línea.

Las pruebas de regresión fallaron antes de aplicar las correcciones.

## Comprobaciones independientes

- 384 autómatas generados (128 de cada tipo), con todas las 63 palabras
  binarias de longitud 0 a 5: **24.192 comparaciones** contra un evaluador
  independiente que explora caminos `(estado, posición)` sin calcular clausuras.
- Los tres ejemplos se contrastan con sus definiciones matemáticas para todas
  las palabras de longitud 0 a 7: **765 casos**. También se probaron esas
  palabras mediante el CLI interactivo compilado.
- Pruebas existentes de parser, validación, entradas, clausura epsilon, ciclos,
  interacción y pipes cerrados.
- CI en GitHub Actions para Linux y Windows, usando Rust 1.88.0, formato,
  Clippy estricto, pruebas y documentación.

No se reprodujo una discrepancia entre las definiciones de los tres ejemplos
y el simulador. Esto no permite atribuir a una causa concreta los problemas
anteriores del usuario sin disponer de su entrada y resultado esperado.

## Límites actuales

- Solo se soportan autómatas finitos `dfa`, `nfa` y `enfa`.
- Los AFD deben ser completos. No se crea automáticamente un estado sumidero.
- Los símbolos numéricos del archivo se escriben entre comillas: `"0"`, `"1"`.
- Para alfabetos con símbolos de varias letras, la entrada se separa por
  espacios; una palabra completa debe ser un único argumento del shell.
- Transformaciones, minimización, conversión a regex e interfaz gráfica siguen
  pendientes.

## Cómo probar

```bash
cargo test --workspace --locked
cargo run -q -p salmon-cli -- run examples/dfa.salmon
```

Con el AFD de ejemplo, `11` devuelve `OK`, `1` devuelve `NOT` y una línea
vacía devuelve `OK`. `:quit` cierra la sesión.
