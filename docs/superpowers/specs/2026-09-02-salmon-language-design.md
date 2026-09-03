# Salmon Language Design

## Objetivo

Salmon será un lenguaje declarativo, compacto e interpretado en Rust para describir autómatas finitos. La primera versión permitirá definir, analizar y validar AFD, AFND y AFND-ε mediante una sintaxis compartida. El modelo público quedará desacoplado del parser para que una interfaz gráfica y futuros algoritmos puedan consumirlo sin conocer la sintaxis fuente.

## Alcance inicial

La primera versión incluye:

- Declaraciones `dfa`, `nfa` y `enfa`.
- Un autómata por archivo.
- Estados, estado inicial, estados finales y transiciones.
- Símbolos de entrada identificadores o literales entre comillas.
- `eps` como símbolo reservado para transiciones vacías.
- Alfabeto inferido de las transiciones o declarado explícitamente.
- Comentarios de línea con `//`.
- Diagnósticos con archivo, línea y columna.
- Una biblioteca Rust para parsear y validar.
- Un comando `salmon check <archivo>` para comprobar archivos `.salmon`.

Quedan fuera de esta primera versión:

- Autómatas de pila, máquinas de Turing, autómatas de Büchi y transductores.
- Simulación interactiva de cadenas.
- Determinización, minimización y conversión a expresiones regulares.
- Importaciones, módulos, variables y operaciones ejecutables dentro del lenguaje.
- Interfaz gráfica y formatos de persistencia adicionales.

## Sintaxis

### AFD

```salmon
dfa ends_with_ab {
    initial: q0
    final: q2

    q0: a=q1, b=q0
    q1: a=q1, b=q2
    q2: a=q1, b=q0
}
```

### AFND-ε

```salmon
enfa example {
    initial: q0
    final: q2

    q0: a=[q0, q1], eps=q2
    q1: b=q2
}
```

### Alfabeto explícito y varios estados finales

```salmon
nfa words {
    alphabet: a, b, "end"
    initial: start
    final: left, right

    start: a=[left, right], "end"=right
}
```

`alphabet` es opcional. Cuando se omite, Salmon lo infiere de todos los símbolos usados en transiciones, excluyendo `eps`. Cuando está presente, usar un símbolo no declarado es un error. Los estados se declaran implícitamente al aparecer como origen o destino de una transición, o al ser mencionados en `initial` o `final`.

## Gramática conceptual

```text
file          := automaton EOF
automaton     := kind identifier "{" item* "}"
kind          := "dfa" | "nfa" | "enfa"
item          := alphabet_decl | initial_decl | final_decl | state_decl
alphabet_decl := "alphabet" ":" symbol ("," symbol)*
initial_decl  := "initial" ":" identifier
final_decl    := "final" ":" identifier ("," identifier)*
state_decl    := identifier ":" transition ("," transition)*
transition    := input "=" destinations
input         := symbol | "eps"
destinations  := identifier | "[" identifier ("," identifier)* "]"
symbol        := identifier | string
```

Los saltos de línea y espacios no son significativos. No se requieren puntos y coma. Las palabras reservadas son `dfa`, `nfa`, `enfa`, `alphabet`, `initial`, `final` y `eps`. Los literales entre comillas permiten usar como símbolo una palabra reservada o un valor con espacios.

## Semántica y validaciones

Reglas comunes:

- Debe existir exactamente una declaración `initial`.
- Debe existir exactamente una declaración `final`, que puede contener uno o más estados.
- Una lista de destinos no puede estar vacía ni repetir estados.
- No puede repetirse una transición para la misma combinación de estado de origen y símbolo.
- `eps` nunca forma parte del alfabeto.
- Una declaración explícita de `alphabet` no puede contener duplicados ni `eps`.

Reglas por tipo:

- `dfa`: no admite `eps`, no admite listas con más de un destino y exige una transición por cada par estado-símbolo del alfabeto.
- `nfa`: no admite `eps`; permite cero, uno o varios destinos por estado-símbolo.
- `enfa`: permite las mismas transiciones que `nfa` y además admite `eps`.

La completitud del AFD se valida después de reunir todos los estados y de inferir o comprobar el alfabeto. Un estado mencionado solamente como destino también debe tener transiciones para cada símbolo en un `dfa`.

## Representación interna

Todos los tipos se normalizan al mismo modelo:

```rust
pub enum AutomatonKind {
    Dfa,
    Nfa,
    Enfa,
}

pub struct Automaton {
    pub name: String,
    pub kind: AutomatonKind,
    pub alphabet: Vec<Symbol>,
    pub states: Vec<StateId>,
    pub initial: StateId,
    pub finals: Vec<StateId>,
    pub transitions: Vec<Transition>,
}

pub struct Transition {
    pub from: StateId,
    pub input: Option<Symbol>,
    pub to: Vec<StateId>,
}
```

`None` representa `eps`. Aunque un AFD tenga un solo destino, el modelo conserva `Vec<StateId>` para compartir estructura con AFND y facilitar transformaciones futuras. La construcción final ordena estados, símbolos, finales y transiciones por su primera aparición para ofrecer resultados deterministas.

## Arquitectura

El proyecto será un workspace Rust con dos crates:

- `salmon-lang`: lexer, parser, AST, análisis semántico, modelo público y función `parse`.
- `salmon-cli`: binario del comando `salmon`, dependiente de `salmon-lang`.

El flujo será:

```text
archivo fuente -> tokens -> AST con spans -> análisis semántico -> Automaton
```

La API principal será:

```rust
pub fn parse(source_name: &str, source: &str) -> Result<Automaton, Vec<Diagnostic>>;
```

El AST conservará posiciones de origen. El modelo `Automaton` no dependerá de tipos del parser, de modo que los consumidores futuros puedan construirlo o utilizarlo directamente.

Se elige un lexer y parser manuales para la gramática inicial. La sintaxis es pequeña y esta opción permite controlar mensajes de error y evitar que la API pública dependa de un generador de parsers. Si la gramática creciera sustancialmente, esta decisión podrá cambiar sin afectar `Automaton` ni `parse`.

## Diagnósticos y recuperación de errores

Cada diagnóstico tendrá un código estable, un mensaje, severidad y rango de origen. Ejemplos:

- `SYN001`: token inesperado.
- `SYN002`: delimitador sin cerrar.
- `SEM001`: falta `initial`.
- `SEM002`: declaración duplicada.
- `SEM003`: símbolo fuera del alfabeto declarado.
- `SEM004`: transición `eps` incompatible con el tipo.
- `SEM005`: AFD no determinista.
- `SEM006`: transición faltante en un AFD.

El parser intentará recuperarse en saltos de línea y cierres `}` para informar varios problemas independientes en una sola ejecución. `salmon check` terminará con código `0` si el archivo es válido, `1` si hay errores del usuario y `2` ante fallos de lectura o uso incorrecto del comando.

## Estrategia de pruebas

- Pruebas unitarias del lexer para identificadores, cadenas, comentarios, puntuación y spans.
- Pruebas unitarias del parser para cada declaración y para recuperación de errores.
- Pruebas semánticas parametrizadas para las reglas comunes y las específicas de cada tipo.
- Pruebas de integración con archivos válidos e inválidos completos.
- Pruebas del CLI para salida, códigos de retorno y errores de lectura.
- Pruebas snapshot solamente para diagnósticos renderizados; los códigos y rangos se comprobarán también con aserciones estructuradas.

## Criterios de aceptación

La primera versión estará completa cuando:

1. Los tres ejemplos de esta especificación produzcan el modelo esperado.
2. Cada regla semántica tenga al menos una prueba positiva o negativa aplicable.
3. Los errores indiquen archivo, línea y columna sin provocar pánicos.
4. `salmon check valid.salmon` termine con `0` y `salmon check invalid.salmon` con `1`.
5. La biblioteca pueda usarse sin depender del CLI.
6. El código supere formato, lint y toda la suite de pruebas del workspace.

## Entregables de documentación

- `docs/diagrams/class-diagram.drawio`: diagrama de clases editable del modelo público y del pipeline interno.
- `docs/diagrams/architecture.drawio`: diagrama de arquitectura editable con biblioteca, CLI y consumidores futuros.
- `docs/user-manual.md`: manual de usuario en español con instalación, sintaxis, ejemplos, validaciones y resolución de errores.
- El nombre se expande oficialmente como **Super Automata Language for Modeling, Operations, and Notation**.
