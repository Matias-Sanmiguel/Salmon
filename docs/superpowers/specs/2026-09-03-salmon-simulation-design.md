# Salmon Simulation and Interactive CLI Design

## Objective

Add a reusable finite-automaton simulation engine to `salmon-lang` and expose it through `salmon run`. Users can test words interactively or in one-shot mode and receive exactly `OK` when the word is accepted or `NOT` when it is rejected.

## Scope

This increment includes:

- Simulation for `dfa`, `nfa`, and `enfa` automata.
- Epsilon closure before consuming input and after every consumed symbol.
- A public, CLI-independent simulation API.
- Interactive `salmon run <file.salmon>` mode.
- One-shot `salmon run <file.salmon> <input>` mode.
- Compact character input and tokenized multi-character input.
- Quoted input symbols containing whitespace.
- Documentation and automated tests.

It does not add tracing, visualization, automaton transformations, regex conversion, persisted sessions, command history, or line editing.

## User Experience

### Interactive mode

```text
$ salmon run examples/dfa.salmon
Salmon — even_ones
> 0011
OK
> 111
NOT
> 
OK
> :quit
```

The banner is written once after the automaton has been parsed successfully. The prompt is `> `. Standard output is flushed before reading each line. A blank line represents the empty word. `:quit` and end-of-file exit successfully.

An invalid automaton produces the existing positioned diagnostics and exits before opening the prompt. An input-format error is printed as `ERROR: <message>` and the interactive session continues. A syntactically valid word containing a symbol outside the alphabet is a rejected word and prints `NOT`.

### One-shot mode

```text
$ salmon run examples/dfa.salmon 0011
OK
$ salmon run examples/dfa.salmon 111
NOT
```

The one-shot command prints one result followed by a newline. Its exit status is:

- `0` when the word is accepted;
- `1` when the word is rejected;
- `2` for incorrect command usage, unreadable files, invalid automata, or malformed input notation.

The existing `salmon check <file.salmon>` behavior and exit codes remain unchanged.

## Input Notation

Salmon converts a CLI input line into `Vec<Symbol>` using deterministic rules:

1. The empty line or empty command argument is the empty word.
2. If every alphabet symbol contains exactly one Unicode scalar and the input contains no whitespace or quotes, the compact form is used: each Unicode scalar becomes one symbol. Thus `abba` means `a b b a`.
3. Otherwise, the tokenized form is used. Tokens are separated by whitespace: `open token close` represents three symbols.
4. Double-quoted tokens preserve embedded whitespace: `open "end value"` represents two symbols.
5. Quoted tokens do not support escape sequences, matching Salmon source strings. An unmatched quote is a malformed input and yields `ERROR`, not `NOT`.
6. `:quit` is a command only when it is the complete interactive line. It can be tested as an alphabet symbol in tokenized form by writing `":quit"`.

When an alphabet mixes single-character and multi-character symbols, compact expansion is disabled. For alphabet `{a, ab}`, input `ab` is the single symbol `ab`; input `a b` is the two symbols `a`, `b` and is rejected if `b` is not in the alphabet.

## Simulation Semantics

The simulator uses a set-of-states algorithm for all three automaton kinds:

1. Start with the singleton set containing `initial`.
2. Compute its epsilon closure.
3. For each input symbol, collect every destination of a matching non-epsilon transition from any active state.
4. Compute epsilon closure of the resulting set.
5. Accept when the final active set intersects `finals`.

The epsilon closure follows `None` transitions transitively until a fixed point. A visited set prevents infinite loops in epsilon cycles. DFA and NFA automata have no epsilon transitions after semantic validation, so the same algorithm works without special cases.

The public API is:

```rust
pub fn accepts(automaton: &Automaton, input: &[Symbol]) -> bool;
```

Unknown input symbols naturally produce an empty active-state set and return `false`. The simulator never mutates the automaton and must not panic for a structurally valid public `Automaton` value.

## Architecture and Data Flow

`crates/salmon-lang/src/simulation.rs` owns the simulation algorithm. It depends only on public model types and is re-exported from `salmon_lang`; it has no CLI or parser dependency.

`crates/salmon-cli/src/input.rs` owns CLI input notation and returns:

```rust
pub(crate) fn parse_input(input: &str, alphabet: &[Symbol]) -> Result<Vec<Symbol>, InputError>;
```

`crates/salmon-cli/src/main.rs` dispatches `check` and `run`, loads the automaton through `salmon_lang::parse`, and calls the input parser and simulator. Interactive I/O is isolated behind a function accepting `BufRead` and `Write` so it can be tested without a real terminal:

```rust
fn run_interactive<R: BufRead, W: Write>(
    automaton: &Automaton,
    reader: R,
    writer: &mut W,
) -> io::Result<()>;
```

The flow is:

```text
file -> parse/validate -> Automaton
line/argument -> parse_input -> Vec<Symbol> -> accepts -> OK | NOT
```

## Error Handling

- File and automaton errors reuse existing `check` rendering.
- `InputError` contains a human-readable message and does not use Salmon source diagnostic codes because it describes command input, not a `.salmon` file.
- Interactive read/write failures terminate the process with exit `2` after a concise stderr message.
- Broken-pipe writes terminate without a Rust panic.
- No result other than semantic acceptance prints `OK`; malformed inputs never masquerade as rejected words.

## Tests

- Unit tests for DFA acceptance/rejection and empty input.
- Unit tests for NFA branching.
- Unit tests for epsilon closure, chained epsilon transitions, and epsilon cycles.
- Unit tests for compact Unicode input, tokenized input, quoted symbols, mixed alphabets, empty input, and unmatched quotes.
- CLI integration tests for accepted/rejected one-shot inputs and exit codes.
- CLI integration test that pipes several lines plus `:quit` into interactive mode and asserts the ordered banner, prompts, and results.
- Regression tests confirming `salmon check` remains unchanged.
- Documentation examples validated through the public API or CLI.

## Acceptance Criteria

1. Each file in `examples/` can be loaded by `salmon run`.
2. Representative accepted and rejected words for all three example automata return the expected result.
3. Empty-word acceptance works through epsilon closure.
4. Interactive mode processes multiple words without restarting.
5. One-shot mode returns `0` for `OK`, `1` for `NOT`, and `2` for operational or notation errors.
6. Existing 37 tests remain green and the new tests cover simulation and CLI behavior.
7. Formatting, strict Clippy, tests, and documentation all pass for the workspace.
