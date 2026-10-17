# Salmon

**Super Automata Language for Modeling, Operations, and Notation**

Salmon is a compact declarative language for describing, validating, and
simulating finite automata.

## Documentation

- [User manual (Spanish)](docs/user-manual.md)
- [Class diagram](docs/diagrams/class-diagram.drawio)
- [Architecture diagram](docs/diagrams/architecture.drawio)
- [Language specification](docs/superpowers/specs/2026-09-02-salmon-language-design.md)
- [MVP implementation plan](docs/superpowers/plans/2026-09-02-salmon-language-mvp.md)

## Quick start

This deterministic finite automaton accepts strings that end in `ab`:

```salmon
dfa ends_with_ab {
    initial: q0
    final: q2

    q0: a=q1, b=q0
    q1: a=q1, b=q2
    q2: a=q1, b=q0
}
```

Build the workspace with Cargo:

```bash
cargo build
```

To install or update the `salmon` command from this checkout:

```bash
cargo install --path crates/salmon-cli --force
salmon run examples/dfa.salmon
```

Run these commands from the repository root. If `salmon` is not found, ensure
Cargo's binary directory (usually `~/.cargo/bin`) is on your `PATH`, or use
the `cargo run` commands below.

Salmon requires Rust 1.88 or newer. Install Rust and Cargo with
[rustup](https://rustup.rs/) before building.

Save an automaton in a `.salmon` file, then validate it:

```bash
cargo run -p salmon-cli -- check path/to/machine.salmon
```

Run one word and print its result:

```bash
cargo run -p salmon-cli -- run examples/dfa.salmon 11
# OK
```

Omit the word to start an interactive session. Enter one word per line and use
`:quit` to leave:

```text
$ cargo run -p salmon-cli -- run examples/dfa.salmon
Salmon — even_ones
> 11
OK
> 1
NOT
> :quit
```

### Input notation and results

When every alphabet symbol is one character, compact input such as `baba` is
read one Unicode character at a time. Otherwise input is whitespace-separated:
`open close` is two symbols, and `open "end marker"` quotes a symbol containing
spaces. Quoted notation also works with single-character alphabets.

A mixed alphabet such as `a, "ab"` always uses tokenized input: `ab` denotes
the single symbol `ab`, while `a ab` denotes two symbols. A blank line in an
interactive session is the empty word; in one-shot mode pass an empty shell
argument (`''`). In one-shot mode, wrap a complete tokenized word containing
spaces as one shell argument, for example `'open "end marker"'`. Exact
unquoted `:quit` ends an interactive session, while `":quit"` is an ordinary
quoted symbol.

One-shot simulation prints `OK` and exits `0` for an accepted word, or prints
`NOT` and exits `1` for a rejected word. Malformed input prints
`ERROR: <message>` to standard error and exits `2`. Invalid command usage,
unreadable files, invalid automata, non-Unicode input, and interactive I/O
failures also exit `2`. Interactive input errors print `ERROR` and the session
continues; `:quit` and end-of-file exit `0`.

The repository examples have these executable results:

| Automaton | Accepted (`OK`) | Rejected (`NOT`) |
| --- | --- | --- |
| `examples/dfa.salmon` | empty word, `11` | `1` |
| `examples/nfa.salmon` | `ab`, `baba` | `aaa` |
| `examples/enfa.salmon` | `a`, `aa` | empty word |

## Language overview

A Salmon file contains one automaton declaration. Its kind is one of:

- `dfa` for a deterministic finite automaton;
- `nfa` for a nondeterministic finite automaton; or
- `enfa` for an NFA that also permits epsilon transitions.

Declarations are compact: whitespace and line breaks are optional, so the
following is valid:

```salmon
nfa choice { initial: start final: left, right start: a=[left, right] }
```

States are introduced by their use. Input symbols may be identifiers or quoted
strings. The optional `alphabet` declaration lists permitted input symbols; if
it is omitted, Salmon infers the alphabet from transitions in first-appearance
order. The reserved `eps` symbol represents an epsilon transition and is never
part of the alphabet. It is allowed only in `enfa` declarations.

```salmon
enfa shortcut {
    alphabet: a, b
    initial: q0
    final: q1
    q0: eps=q1, a=q0
    q1: b=q1
}
```

## Scope

This MVP validates finite-automaton definitions and simulates input words.
Transformations (including determinization and minimization), regex conversion,
and a UI remain future work.
