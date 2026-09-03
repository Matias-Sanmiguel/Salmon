# Salmon

Salmon (Super Automata Language for Modeling, Operations, and Notation) is a
compact declarative language for describing and validating finite automata.

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

Salmon requires Rust 1.88 or newer. Install Rust and Cargo with
[rustup](https://rustup.rs/) before building.

Save an automaton in a `.salmon` file, then validate it:

```bash
cargo run -p salmon-cli -- check path/to/machine.salmon
```

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

This MVP validates finite-automaton definitions. Simulation, transformations
(including determinization and minimization), regex conversion, and a UI are
future work.
