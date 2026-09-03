# Salmon Simulation CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reusable DFA/NFA/ε-NFA simulation plus interactive and one-shot `salmon run` commands that print `OK` or `NOT`.

**Architecture:** `salmon-lang` gains a model-only simulation module using a state-set and epsilon-closure algorithm. `salmon-cli` gains a separate input-notation parser, while `main.rs` owns command dispatch, file loading, exit codes, and generic interactive I/O.

**Tech Stack:** Rust 1.88.0, edition 2024, standard library only, Cargo workspace.

**Spec:** `docs/superpowers/specs/2026-09-03-salmon-simulation-design.md`

## Global Constraints

- Preserve `salmon check <file.salmon>` output and exit codes exactly.
- Print exactly `OK` for accepted words and `NOT` for rejected words.
- Use exit `0` for accepted one-shot input, `1` for rejected input, and `2` for usage, I/O, invalid automata, or malformed input notation.
- Treat an empty interactive line or empty one-shot argument as the empty word.
- Compute epsilon closure before input and after every consumed symbol; terminate safely on epsilon cycles.
- Keep simulation independent of parser and CLI modules.
- Support compact Unicode-character input only when every alphabet symbol is one scalar and the input has no whitespace or quotes.
- Use tokenized input otherwise, with whitespace separators and double-quoted tokens; do not support escapes.
- Remain dependency-free outside the Rust standard library.
- Follow strict TDD for production behavior and keep all existing tests green.

---

## File Map

```text
crates/salmon-lang/src/simulation.rs       reusable state-set simulator
crates/salmon-lang/src/lib.rs              exports `accepts`
crates/salmon-cli/src/input.rs             CLI word-notation parser
crates/salmon-cli/src/main.rs              check/run dispatch and interactive loop
crates/salmon-cli/tests/check.rs            existing check regression tests
crates/salmon-cli/tests/run.rs              one-shot and interactive process tests
README.md                                   simulation quick start
docs/user-manual.md                         complete run/input documentation
docs/diagrams/class-diagram.drawio          simulation API and input-parser classes
docs/diagrams/architecture.drawio           run data flow and current simulation component
```

### Task 1: Reusable finite-automaton simulator

**Files:**
- Create: `crates/salmon-lang/src/simulation.rs`
- Modify: `crates/salmon-lang/src/lib.rs`

**Interfaces:**
- Consumes: `Automaton`, `StateId`, `Symbol`, and `Transition`.
- Produces: `pub fn accepts(automaton: &Automaton, input: &[Symbol]) -> bool`.

- [ ] **Step 1: Write failing simulation tests**

Add unit tests in `simulation.rs` using a helper that calls the existing public `parse` function. Cover:

```rust
#[test]
fn simulates_a_dfa() {
    let machine = machine("dfa even { initial:q0 final:q0 q0:a=q1 q1:a=q0 }");
    assert!(accepts(&machine, &[]));
    assert!(accepts(&machine, &[symbol("a"), symbol("a")]));
    assert!(!accepts(&machine, &[symbol("a")]));
}

#[test]
fn simulates_nondeterministic_branches() {
    let machine = machine("nfa x { initial:q0 final:q2 q0:a=[q0,q1] q1:b=q2 }");
    assert!(accepts(&machine, &[symbol("a"), symbol("a"), symbol("b")]));
    assert!(!accepts(&machine, &[symbol("a"), symbol("a")]));
}

#[test]
fn computes_transitive_epsilon_closure_and_stops_on_cycles() {
    let machine = machine(
        "enfa x { initial:q0 final:q2 q0:eps=q1 q1:eps=[q0,q2] }",
    );
    assert!(accepts(&machine, &[]));
}

#[test]
fn unknown_symbols_are_rejected() {
    let machine = machine("nfa x { initial:q0 final:q1 q0:a=q1 }");
    assert!(!accepts(&machine, &[symbol("unknown")]));
}
```

- [ ] **Step 2: Run focused tests and observe RED**

Run: `cargo test -p salmon-lang simulation::tests`

Expected: compilation fails because `simulation` and `accepts` do not exist.

- [ ] **Step 3: Implement the state-set algorithm**

Use `HashSet<StateId>` and a stack-based helper:

```rust
fn epsilon_closure(automaton: &Automaton, seeds: HashSet<StateId>) -> HashSet<StateId> {
    let mut closure = seeds;
    let mut stack: Vec<StateId> = closure.iter().cloned().collect();
    while let Some(state) = stack.pop() {
        for transition in automaton.transitions.iter()
            .filter(|t| t.from == state && t.input.is_none())
        {
            for destination in &transition.to {
                if closure.insert(destination.clone()) {
                    stack.push(destination.clone());
                }
            }
        }
    }
    closure
}
```

`accepts` initializes the closure from `initial`, folds each input symbol through matching `Some(symbol)` transitions plus a fresh closure, and finally checks intersection with `finals`. Re-export `accepts` from `lib.rs`.

- [ ] **Step 4: Run focused and full library tests**

Run: `cargo test -p salmon-lang simulation::tests && cargo test -p salmon-lang && cargo clippy -p salmon-lang --all-targets -- -D warnings`

Expected: all simulation tests and the existing language suite pass.

- [ ] **Step 5: Commit the simulator**

```bash
git add crates/salmon-lang/src
git commit -m "feat: simulate finite automata"
```

### Task 2: CLI input-notation parser

**Files:**
- Create: `crates/salmon-cli/src/input.rs`
- Modify: `crates/salmon-cli/src/main.rs`

**Interfaces:**
- Consumes: `salmon_lang::Symbol`.
- Produces: `InputError(String)` implementing `Display` and `Error`.
- Produces: `parse_input(input: &str, alphabet: &[Symbol]) -> Result<Vec<Symbol>, InputError>`.

- [ ] **Step 1: Add failing input-parser tests**

```rust
#[test]
fn compact_input_splits_unicode_scalars_for_a_single_character_alphabet() {
    let alphabet = symbols(&["a", "ñ"]);
    assert_eq!(parse_input("añ", &alphabet).unwrap(), symbols(&["a", "ñ"]));
}

#[test]
fn tokenized_input_supports_multiple_and_quoted_symbols() {
    let alphabet = symbols(&["open", "end value"]);
    assert_eq!(
        parse_input("open \"end value\"", &alphabet).unwrap(),
        symbols(&["open", "end value"]),
    );
}

#[test]
fn mixed_alphabets_disable_compact_expansion() {
    let alphabet = symbols(&["a", "ab"]);
    assert_eq!(parse_input("ab", &alphabet).unwrap(), symbols(&["ab"]));
    assert_eq!(parse_input("a ab", &alphabet).unwrap(), symbols(&["a", "ab"]));
}

#[test]
fn empty_input_is_the_empty_word() {
    assert!(parse_input("", &symbols(&["a"])).unwrap().is_empty());
}

#[test]
fn rejects_unclosed_quotes() {
    let error = parse_input("open \"end value", &symbols(&["open"])).unwrap_err();
    assert_eq!(error.to_string(), "unclosed quoted symbol");
}

#[test]
fn quoted_quit_is_a_regular_symbol() {
    assert_eq!(parse_input("\":quit\"", &symbols(&[":quit"])).unwrap(), symbols(&[":quit"]));
}
```

- [ ] **Step 2: Run tests and observe RED**

Run: `cargo test -p salmon-cli input::tests`

Expected: compilation fails because the input module and parser do not exist.

- [ ] **Step 3: Implement compact and tokenized parsing**

Compact mode applies only when every `Symbol.0.chars().count() == 1` and the input contains neither whitespace nor `"`. Otherwise scan tokens left to right. Skip Unicode whitespace; an opening `"` consumes through the next `"`; an unquoted token consumes until whitespace; any quote inside an unquoted token or non-whitespace immediately after a closing quote returns `InputError("unexpected quote in input")`.

- [ ] **Step 4: Run focused tests and strict lint**

Run: `cargo test -p salmon-cli input::tests && cargo clippy -p salmon-cli --all-targets -- -D warnings`

Expected: all input tests pass without warnings.

- [ ] **Step 5: Commit the input parser**

```bash
git add crates/salmon-cli/src
git commit -m "feat: parse Salmon CLI input words"
```

### Task 3: One-shot `salmon run`

**Files:**
- Modify: `crates/salmon-cli/src/main.rs`
- Create: `crates/salmon-cli/tests/run.rs`

**Interfaces:**
- Consumes: `salmon_lang::{parse, accepts}` and `input::parse_input`.
- Produces: `salmon run <file.salmon> <input>` with output and exit codes from the specification.

- [ ] **Step 1: Add failing one-shot process tests**

Use the repository examples resolved from `env!("CARGO_MANIFEST_DIR")/../../examples`:

```rust
#[test]
fn one_shot_returns_ok_and_zero_for_an_accepted_word() {
    let output = salmon().args(["run", dfa(), "11"]).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"OK\n");
}

#[test]
fn one_shot_returns_not_and_one_for_a_rejected_word() {
    let output = salmon().args(["run", dfa(), "1"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"NOT\n");
}

#[test]
fn malformed_input_and_invalid_automata_exit_two() {
    let malformed = salmon().args(["run", dfa(), "\"open"]).output().unwrap();
    assert_eq!(malformed.status.code(), Some(2));
    let invalid = salmon().args(["run", invalid_fixture(), "a"]).output().unwrap();
    assert_eq!(invalid.status.code(), Some(2));
}
```

Add accepted/rejected one-shot cases for `nfa.salmon` (`ab`/`aaa`) and `enfa.salmon` (`aa`/empty argument).

- [ ] **Step 2: Run one-shot tests and observe RED**

Run: `cargo test -p salmon-cli --test run one_shot`

Expected: commands return usage exit `2` because `run` is not implemented.

- [ ] **Step 3: Refactor file loading without changing `check`**

Extract a helper returning either `Automaton` or a rendered error status. Preserve the existing `check` language-error exit `1`; pass exit `2` when the same invalid automaton is loaded by `run`. Update usage to:

```text
usage:
  salmon check <file.salmon>
  salmon run <file.salmon> [input]
```

- [ ] **Step 4: Implement one-shot dispatch**

Accept exactly three post-program arguments for one-shot mode. Require the input argument to be valid Unicode, call `parse_input`, then `accepts`. Print one line and return `ExitCode::SUCCESS` for acceptance or `ExitCode::from(1)` for rejection. Print `ERROR: <message>` to stderr and return `2` for malformed notation.

- [ ] **Step 5: Verify one-shot and check regression tests**

Run: `cargo test -p salmon-cli --test run && cargo test -p salmon-cli --test check`

Expected: all new run cases pass and all four existing check tests remain unchanged.

- [ ] **Step 6: Commit one-shot run mode**

```bash
git add crates/salmon-cli/src/main.rs crates/salmon-cli/tests/run.rs
git commit -m "feat: run automata from the CLI"
```

### Task 4: Interactive `salmon run`

**Files:**
- Modify: `crates/salmon-cli/src/main.rs`
- Modify: `crates/salmon-cli/tests/run.rs`

**Interfaces:**
- Produces: `run_interactive<R: BufRead, W: Write>(automaton, reader, writer) -> io::Result<()>`.
- Produces: `salmon run <file.salmon>` REPL with banner, prompt, multiple results, input errors, `:quit`, and EOF.

- [ ] **Step 1: Add failing interactive process tests**

Spawn `salmon run examples/dfa.salmon` with piped stdin/stdout, write `11\n1\n\n:quit\n`, and assert exact stdout:

```text
Salmon — even_ones
> OK
> NOT
> OK
> 
```

The exact byte string ends with the final prompt and no trailing newline:

```rust
b"Salmon \u{2014} even_ones\n> OK\n> NOT\n> OK\n> "
```

Add a second case writing `"unterminated\n11\n:quit\n`; assert `ERROR: unclosed quoted symbol` appears and the following valid line still prints `OK`.

- [ ] **Step 2: Run interactive tests and observe RED**

Run: `cargo test -p salmon-cli --test run interactive`

Expected: two-argument `run` returns usage exit `2` instead of starting a session.

- [ ] **Step 3: Implement generic interactive I/O**

Write the banner, then loop: write and flush `> `, read one line, stop on EOF, remove `\n` and optional preceding `\r`, stop on exact `:quit`, parse the line, and print `OK`, `NOT`, or `ERROR: <message>`. Propagate I/O errors from the generic helper; translate them to a concise stderr error and exit `2` in `main`.

- [ ] **Step 4: Verify interactive and complete CLI suites**

Run: `cargo test -p salmon-cli --test run && cargo test -p salmon-cli --test check && cargo clippy -p salmon-cli --all-targets -- -D warnings`

Expected: all interactive/one-shot tests and existing check tests pass.

- [ ] **Step 5: Commit interactive mode**

```bash
git add crates/salmon-cli/src/main.rs crates/salmon-cli/tests/run.rs
git commit -m "feat: add interactive automaton runner"
```

### Task 5: Documentation, diagrams, and end-to-end verification

**Files:**
- Modify: `README.md`
- Modify: `docs/user-manual.md`
- Modify: `docs/diagrams/class-diagram.drawio`
- Modify: `docs/diagrams/architecture.drawio`
- Modify: `crates/salmon-cli/tests/run.rs`

**Interfaces:**
- Documents: both run modes, input notation, empty word, results, errors, and exits.
- Verifies: all three repository examples in accepted and rejected cases.

- [ ] **Step 1: Add an end-to-end example matrix**

Add a process-test table with these exact expectations:

```text
examples/dfa.salmon:  "" -> OK, "11" -> OK, "1" -> NOT
examples/nfa.salmon:  "ab" -> OK, "baba" -> OK, "aaa" -> NOT
examples/enfa.salmon: "a" -> OK, "aa" -> OK, "" -> NOT
```

Run: `cargo test -p salmon-cli --test run example_matrix`

Expected before completing documentation work: the matrix passes and becomes the executable source for documented results.

- [ ] **Step 2: Update README and Spanish manual**

Add quick-start commands for interactive and one-shot modes. Document compact vs tokenized input, quoted symbols, mixed alphabets, blank-line epsilon, `:quit`, `ERROR`, and exit statuses. State that simulation is now implemented while transformations, regex conversion, and UI remain future work.

- [ ] **Step 3: Update both editable Draw.io diagrams**

Add the public `accepts` API and simulation module to the class diagram. Add `input -> parse_input -> accepts -> OK/NOT`, interactive and one-shot CLI paths, and the now-implemented simulation component to the architecture diagram. Preserve uncompressed editable XML, existing visual conventions, and current/future legend semantics.

- [ ] **Step 4: Validate documentation and Draw.io structure**

Run:

```bash
python3 - <<'PY'
from pathlib import Path
from xml.etree import ElementTree
for path in Path('docs/diagrams').glob('*.drawio'):
    root = ElementTree.parse(path).getroot()
    assert root.tag == 'mxfile'
    assert root.find('./diagram/mxGraphModel/root') is not None
print('validated 2 drawio files')
PY
rg -n 'salmon run|OK|NOT|:quit' README.md docs/user-manual.md
```

Expected: two editable diagrams validate and both documents cover the interactive command vocabulary.

- [ ] **Step 5: Run every final quality gate**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
git diff --check
```

Expected: every command exits `0`; the original 37 tests and all new simulation/input/CLI tests pass.

- [ ] **Step 6: Commit documentation and final verification**

```bash
git add README.md docs crates/salmon-cli/tests/run.rs
git commit -m "docs: document interactive Salmon simulation"
```

## Completion Evidence

Before declaring the increment complete, record fresh output from the full quality gate, validate both Draw.io files, manually pipe several words through the interactive CLI, and confirm `git status --short` is clean.
