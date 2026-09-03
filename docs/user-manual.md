# Manual de usuario de Salmon

**Salmon** significa **Super Automata Language for Modeling, Operations, and Notation**. Es un lenguaje declarativo para describir y validar autómatas finitos. En esta primera versión se validan AFD, AFND y AFND-ε; Salmon no simula cadenas ni transforma autómatas.

## Requisitos e instalación

Necesita Rust 1.88 o una versión posterior, incluido Cargo. La forma
recomendada de instalarlo es [rustup](https://rustup.rs/). Desde la raíz del
proyecto, compile el espacio de trabajo:

```bash
cargo build
```

Para usar el ejecutable desde el código fuente no hace falta instalarlo en el
sistema:

```bash
cargo run -p salmon-cli -- check ruta/al/automata.salmon
```

También puede ejecutar directamente el binario generado por Cargo, normalmente
en `target/debug/salmon`:

```bash
target/debug/salmon check ruta/al/automata.salmon
```

El único subcomando del MVP es `check`. Recibe exactamente un archivo:

```text
salmon check <archivo.salmon>
```

El comando no exige la extensión `.salmon`, pero usarla facilita identificar
los archivos fuente. Su código de salida es:

| Código | Significado |
| --- | --- |
| `0` | El archivo es sintáctica y semánticamente válido. |
| `1` | El archivo contiene uno o más errores de Salmon. |
| `2` | El uso del comando es incorrecto o el archivo no se puede leer. |

## Primer AFD

Este AFD acepta palabras que terminan en `ab`. Guárdelo, por ejemplo, como
`ends_with_ab.salmon` y compruébelo con `salmon check ends_with_ab.salmon`.

```salmon
dfa ends_with_ab {
    initial: q0
    final: q2

    q0: a=q1, b=q0
    q1: a=q1, b=q2
    q2: a=q1, b=q0
}
```

El tipo `dfa` declara un AFD. `initial` nombra el estado inicial, `final` uno
o más estados de aceptación y cada fila de estado enumera sus transiciones.
En este caso el alfabeto se infiere como `a, b` a partir de las transiciones.

## Gramática del MVP

Un archivo contiene exactamente una declaración de autómata. Los espacios, las
tabulaciones y los saltos de línea no son significativos entre elementos: no
hay punto y coma ni separador de filas obligatorio.

```text
archivo          := automata EOF
automata         := tipo identificador "{" elemento* "}"
tipo             := "dfa" | "nfa" | "enfa"
elemento         := alfabeto | inicial | finales | fila_estado
alfabeto         := "alphabet" ":" simbolo ("," simbolo)*
inicial          := "initial" ":" identificador
finales          := "final" ":" identificador ("," identificador)*
fila_estado      := identificador ":" transicion ("," transicion)*
transicion       := entrada "=" destinos
entrada          := simbolo | "eps"
destinos         := identificador | "[" identificador ("," identificador)* "]"
simbolo          := identificador | cadena
identificador    := "_" o letra, seguido de "_", letras o dígitos
cadena           := caracteres sin salto de línea entre comillas dobles
```

Las palabras `dfa`, `nfa`, `enfa`, `alphabet`, `initial`, `final` y `eps` son
reservadas. Los identificadores de estados, los nombres de autómatas y los
destinos deben ser identificadores sin comillas. Las cadenas entre comillas se
usan solo como símbolos de entrada; no admiten secuencias de escape ni pueden
contener una comilla doble o un salto de línea.

Los comentarios empiezan con `//` y terminan al final de la línea:

```salmon
nfa comments {
    // Se ignora todo este texto.
    initial: start
    final: accept // También puede ir al final de una declaración.
    start: token=accept
}
```

## Tipos de autómata y destinos

### AFD (`dfa`)

Un AFD no permite `eps`, ni varios destinos en una misma transición. Además,
debe tener una transición para cada combinación de estado y símbolo del
alfabeto. Salmon comprueba esta completitud incluso para un estado que aparezca
solo como destino.

```salmon
dfa toggle {
    initial: off
    final: on
    off: press=on
    on: press=off
}
```

### AFND (`nfa`)

Un AFND permite una lista de destinos para una transición y puede omitir
transiciones. No admite `eps`.

```salmon
nfa choice {
    initial: start
    final: left, right
    start: a=[left, right]
}
```

### AFND-ε (`enfa`)

Un AFND-ε ofrece las mismas posibilidades que un AFND y además admite la
entrada reservada `eps`. `eps` representa una transición vacía y no forma
parte del alfabeto.

```salmon
enfa shortcut {
    alphabet: a, b
    initial: q0
    final: q1
    q0: eps=q1, a=q0
    q1: b=q1
}
```

## Alfabeto, símbolos y estados

La declaración `alphabet` es opcional. Si se omite, Salmon forma el alfabeto
con los símbolos usados en las transiciones, conservando el orden de primera
aparición. Cuando hay una o más declaraciones `alphabet`, toda transición con
símbolo de entrada debe usar un símbolo de esas listas; un símbolo extra es un
error.

```salmon
nfa words {
    alphabet: a, b, "end marker"
    initial: start
    final: accepted
    start: a=[start, accepted], "end marker"=accepted
}
```

Los símbolos pueden ser identificadores (`a`, `token_2`) o cadenas entre
comillas (`"end marker"`, `"final"`). Las cadenas permiten espacios y usar
texto que, sin comillas, sería una palabra reservada. El `eps` sin comillas es
sintaxis reservada para transiciones de un `enfa`; si se necesita el texto
literal como símbolo ordinario, escríbalo como `"eps"`.

No hay una declaración separada de estados. Un estado se incorpora al modelo
al aparecer en `initial`, `final`, como origen de una fila o como destino. Una
fila de estado contiene una o más transiciones; no existe una sintaxis para
declarar un estado aislado.

## Reglas de validación

Salmon verifica, entre otras, las siguientes reglas:

- Debe haber una declaración `initial` y una declaración `final`; repetir una
  de ellas produce un error.
- Un `final` puede listar uno o más estados separados por comas.
- No se puede repetir una transición del mismo estado de origen con la misma
  entrada, ni repetir un destino dentro de su lista.
- No se pueden repetir símbolos dentro del alfabeto explícito.
- `dfa` y `nfa` rechazan transiciones `eps`; solo `enfa` las admite.
- En un `dfa`, una transición tiene exactamente un destino y el autómata debe
  estar completo respecto de su alfabeto.

## Cómo leer los errores

Los diagnósticos se escriben en la salida de error con este formato:

```text
archivo.salmon:línea:columna: error[CÓDIGO]: explicación
```

Por ejemplo, una transición epsilon en un AFD puede informar:

```text
machine.salmon:4:9: error[SEM004]: epsilon transitions are only allowed in enfa
```

Los códigos distinguen la categoría del problema:

| Código | Interpretación |
| --- | --- |
| `SYN001` | Carácter, token o estructura inesperados. |
| `SYN002` | Cadena, llave o lista de destinos sin cerrar. |
| `SEM001` | Falta `initial` o `final`. |
| `SEM002` | Declaración, símbolo, transición o destino duplicado. |
| `SEM003` | Se usó un símbolo fuera del alfabeto explícito. |
| `SEM004` | Se usó `eps` en un tipo que no es `enfa`. |
| `SEM005` | Un AFD tiene una transición con más de un destino. |
| `SEM006` | Falta una transición requerida para completar un AFD. |

Algunos errores de sintaxis se recuperan en un salto de línea o en `}` para
informar varios problemas en una sola ejecución. Corrija primero el error más
cercano a la parte superior del archivo, ya que una estructura incompleta
puede ocasionar mensajes posteriores derivados.

## Resolución de problemas

| Situación | Revise esto |
| --- | --- |
| El comando muestra `usage: salmon check <file.salmon>` | Use exactamente `check` y una ruta de archivo. |
| El comando termina con `2` y “could not read” | Compruebe la ruta, permisos y que el archivo exista. |
| `SYN001` aparece al usar un estado entre comillas | Los estados deben ser identificadores; las comillas son solo para símbolos. |
| `SYN002` menciona una cadena sin cerrar | Cierre las comillas en la misma línea; no hay escapes de comillas. |
| `SEM003` aparece tras declarar `alphabet` | Añada el símbolo usado a `alphabet` o ajuste la transición. |
| `SEM004` aparece en un `dfa` o `nfa` | Quite `eps` o cambie el tipo a `enfa` si describe una transición epsilon. |
| `SEM005` aparece en un `dfa` | Reemplace `[q1, q2]` por un único destino. |
| `SEM006` aparece en un `dfa` | Añada la transición faltante para cada estado y símbolo del alfabeto. |

## Límites de esta versión

El MVP solo analiza y valida definiciones. No incluye simulación de cadenas,
determinización, minimización, conversión a expresiones regulares, interfaz
gráfica, importaciones, módulos, variables ni operaciones ejecutables. Para
el diseño completo del lenguaje y del modelo, consulte la
[especificación](superpowers/specs/2026-09-02-salmon-language-design.md).

## Diagramas y referencias

- [Diagrama de clases](diagrams/class-diagram.drawio): modelo público y
  componentes internos.
- [Diagrama de arquitectura](diagrams/architecture.drawio): relación entre la
  biblioteca, la CLI y consumidores futuros.
- [README del proyecto](../README.md): inicio rápido y resumen en inglés.
