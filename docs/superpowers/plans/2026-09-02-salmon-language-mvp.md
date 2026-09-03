# Salmon Language MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust library and CLI that parse and validate compact `.salmon` definitions for DFA, NFA, and ε-NFA automata.

**Architecture:** A dependency-free Rust workspace separates the reusable `salmon-lang` library from the `salmon-cli` binary. Source flows through a hand-written lexer, an AST-producing recursive-descent parser, semantic analysis, and a parser-independent `Automaton` model.

**Tech Stack:** Rust 1.88.0, Rust edition 2024, Cargo workspace, Rust standard library only.

**Spec:** `docs/superpowers/specs/2026-09-02-salmon-language-design.md`

## Global Constraints

- Support exactly one `dfa`, `nfa`, or `enfa` declaration per file.
- Keep `alphabet` optional and infer it in first-appearance order when omitted.
- Treat `eps` as reserved syntax and represent it as `None`, never as an alphabet symbol.
- Keep the public automaton model independent of lexer, parser, and AST types.
- Report every user-facing error with a stable code, source name, line, and column; malformed input must not panic.
- Use no runtime or development dependencies outside the Rust standard library for the MVP.
- Require Rust 1.88.0 and edition 2024.
- Follow TDD: observe each focused test fail before adding its minimal implementation.

---

## File Map

```text
Cargo.toml                                      workspace members and shared package settings
crates/salmon-lang/Cargo.toml                   reusable language crate manifest
crates/salmon-lang/src/lib.rs                   public parse API and exports
crates/salmon-lang/src/model.rs                 parser-independent automaton model
crates/salmon-lang/src/diagnostic.rs            spans, stable errors, and text rendering
crates/salmon-lang/src/lexer.rs                 tokens and source scanner
crates/salmon-lang/src/ast.rs                   private syntax tree with source spans
crates/salmon-lang/src/parser.rs                recursive-descent grammar parser
crates/salmon-lang/src/semantic.rs              validation and AST-to-model normalization
crates/salmon-lang/tests/language.rs            public API integration tests
crates/salmon-cli/Cargo.toml                    CLI crate manifest
crates/salmon-cli/src/main.rs                   `salmon check` command
crates/salmon-cli/tests/check.rs                 process-level CLI tests
crates/salmon-cli/tests/fixtures/valid.salmon   valid CLI fixture
crates/salmon-cli/tests/fixtures/invalid.salmon invalid CLI fixture
README.md                                       syntax and command quick start
```

### Task 1: Workspace and public automaton model

**Files:**
- Create: `Cargo.toml`
- Create: `crates/salmon-lang/Cargo.toml`
- Create: `crates/salmon-lang/src/lib.rs`
- Create: `crates/salmon-lang/src/model.rs`

**Interfaces:**
- Produces: `AutomatonKind`, `StateId`, `Symbol`, `Transition`, and `Automaton`.
- Produces later: `parse(source_name: &str, source: &str) -> Result<Automaton, Vec<Diagnostic>>`; use a temporary stub only until Task 6.

- [ ] **Step 1: Create the workspace manifests, CLI stub, and a failing model test**

```toml
# Cargo.toml
[workspace]
members = ["crates/salmon-lang", "crates/salmon-cli"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.88"
license = "MIT"
```

```toml
# crates/salmon-lang/Cargo.toml
[package]
name = "salmon-lang"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[lib]
name = "salmon_lang"
path = "src/lib.rs"
```

Also create `crates/salmon-cli/Cargo.toml` using the final manifest shown in Task 7 and create `crates/salmon-cli/src/main.rs` containing only:

```rust
fn main() {}
```

Create `crates/salmon-lang/src/lib.rs` with `pub mod model;`. Add this test at the bottom of `model.rs` before defining the types:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_can_represent_epsilon_and_multiple_destinations() {
        let transition = Transition {
            from: StateId("q0".into()),
            input: None,
            to: vec![StateId("q0".into()), StateId("q1".into())],
        };

        assert_eq!(transition.input, None);
        assert_eq!(transition.to.len(), 2);
    }
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test -p salmon-lang model::tests::transition_can_represent_epsilon_and_multiple_destinations`

Expected: compilation fails because `Transition` and `StateId` do not exist.

- [ ] **Step 3: Implement the public model**

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomatonKind { Dfa, Nfa, Enfa }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Symbol(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub from: StateId,
    pub input: Option<Symbol>,
    pub to: Vec<StateId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Automaton {
    pub name: String,
    pub kind: AutomatonKind,
    pub alphabet: Vec<Symbol>,
    pub states: Vec<StateId>,
    pub initial: StateId,
    pub finals: Vec<StateId>,
    pub transitions: Vec<Transition>,
}
```

- [ ] **Step 4: Run the focused test and workspace check**

Run: `cargo test -p salmon-lang model::tests::transition_can_represent_epsilon_and_multiple_destinations && cargo check --workspace`

Expected: the model test passes and the complete workspace compiles.

- [ ] **Step 5: Commit the model foundation**

```bash
git add Cargo.toml crates/salmon-lang crates/salmon-cli/Cargo.toml crates/salmon-cli/src/main.rs
git commit -m "feat: establish Salmon workspace and automaton model"
```

### Task 2: Source spans, diagnostics, and lexer

**Files:**
- Create: `crates/salmon-lang/src/diagnostic.rs`
- Create: `crates/salmon-lang/src/lexer.rs`
- Modify: `crates/salmon-lang/src/lib.rs`

**Interfaces:**
- Produces: `Span { start: usize, end: usize }`, `Severity::Error`, and `Diagnostic { code: &'static str, message: String, severity: Severity, span: Span }`.
- Produces: `render_diagnostic(source_name: &str, source: &str, diagnostic: &Diagnostic) -> String`.
- Produces internally: `Token { kind: TokenKind, span: Span }` and `lex(source) -> (Vec<Token>, Vec<Diagnostic>)`.

- [ ] **Step 1: Write lexer tests before the implementation**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_keywords_symbols_strings_comments_and_punctuation() {
        let source = "dfa machine {\n// ignored\nq0: a=q1, \"end value\"=[q1]\n}";
        let (tokens, errors) = lex(source);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Dfa);
        assert!(tokens.iter().any(|t| t.kind == TokenKind::String("end value".into())));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Newline));
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn reports_unexpected_characters_without_panicking() {
        let (_, errors) = lex("dfa x { q0: a=@ }");
        assert_eq!(errors[0].code, "SYN001");
        assert_eq!(errors[0].span, Span { start: 14, end: 15 });
    }

    #[test]
    fn reports_an_unclosed_string() {
        let (_, errors) = lex("dfa x { alphabet: \"end }");
        assert_eq!(errors[0].code, "SYN002");
    }
}
```

- [ ] **Step 2: Run the lexer tests and verify they fail**

Run: `cargo test -p salmon-lang lexer::tests`

Expected: compilation fails because the lexer module and types do not exist.

- [ ] **Step 3: Implement diagnostic primitives and rendering**

Use byte offsets in `Span`. Implement line and column by scanning `source[..span.start]`; columns count Unicode scalar values, starting at 1. The rendered form must be exactly:

```text
machine.salmon:3:5: error[SEM004]: eps is not allowed in dfa
```

Add a diagnostic unit test:

```rust
#[test]
fn renders_source_position() {
    let diagnostic = Diagnostic::error("SEM004", "eps is not allowed in dfa", Span::new(10, 13));
    assert_eq!(
        render_diagnostic("machine.salmon", "dfa x {\n  eps\n}", &diagnostic),
        "machine.salmon:2:3: error[SEM004]: eps is not allowed in dfa"
    );
}
```

- [ ] **Step 4: Implement the scanner**

Define `TokenKind` exactly as `Identifier(String)`, `String(String)`, `Dfa`, `Nfa`, `Enfa`, `Alphabet`, `Initial`, `Final`, `Epsilon`, `LeftBrace`, `RightBrace`, `LeftBracket`, `RightBracket`, `Colon`, `Comma`, `Equals`, `Newline`, and `Eof`. Scan left to right; skip spaces and `//` comments but emit `Newline` tokens. On an invalid character, emit `SYN001`, advance by one Unicode scalar, and continue. On an unclosed string, emit `SYN002`, consume to newline or EOF, and continue.

- [ ] **Step 5: Run tests and lint**

Run: `cargo test -p salmon-lang && cargo clippy -p salmon-lang --all-targets -- -D warnings`

Expected: all focused tests pass and Clippy emits no warnings.

- [ ] **Step 6: Commit diagnostics and lexing**

```bash
git add crates/salmon-lang/src
git commit -m "feat: tokenize Salmon source with diagnostics"
```

### Task 3: Spanned AST and recursive-descent parser

**Files:**
- Create: `crates/salmon-lang/src/ast.rs`
- Create: `crates/salmon-lang/src/parser.rs`
- Modify: `crates/salmon-lang/src/lib.rs`

**Interfaces:**
- Consumes: `Token`, `TokenKind`, `Span`, and `Diagnostic` from Task 2.
- Produces internally: `AstAutomaton`, `AstItem`, `AstState`, `AstTransition`, `AstInput`, and `Spanned<T>`.
- Produces internally: `parse_tokens(tokens: &[Token]) -> Result<AstAutomaton, Vec<Diagnostic>>`.

- [ ] **Step 1: Define parser expectations as failing tests**

```rust
#[test]
fn parses_compact_state_rows_and_multiple_destinations() {
    let source = "enfa example { initial: q0 final: q2 q0: a=[q0, q1], eps=q2 }";
    let (tokens, lex_errors) = lex(source);
    assert!(lex_errors.is_empty());
    let ast = parse_tokens(&tokens).unwrap();
    assert_eq!(ast.name.value, "example");
    let state = ast.items.iter().find_map(|item| match item {
        AstItem::State(state) => Some(state),
        _ => None,
    }).unwrap();
    assert_eq!(state.transitions.len(), 2);
    assert_eq!(state.transitions[0].destinations.len(), 2);
}

#[test]
fn rejects_a_second_automaton() {
    let (tokens, _) = lex("dfa a {} nfa b {}");
    let errors = parse_tokens(&tokens).unwrap_err();
    assert_eq!(errors[0].code, "SYN001");
}

#[test]
fn recovers_at_a_newline_and_reports_multiple_errors() {
    let (tokens, _) = lex("dfa x {\ninitial q0\nfinal q1\n}");
    let errors = parse_tokens(&tokens).unwrap_err();
    assert_eq!(errors.len(), 2);
}

#[test]
fn reports_unclosed_braces_and_destination_lists() {
    for source in ["dfa x {", "nfa x { q0: a=[q1 }"] {
        let (tokens, _) = lex(source);
        let errors = parse_tokens(&tokens).unwrap_err();
        assert!(errors.iter().any(|error| error.code == "SYN002"));
    }
}
```

- [ ] **Step 2: Run parser tests and verify they fail**

Run: `cargo test -p salmon-lang parser::tests`

Expected: compilation fails because AST and parser types do not exist.

- [ ] **Step 3: Implement the private AST**

Use these shapes and keep them `pub(crate)`:

```rust
pub(crate) struct Spanned<T> { pub value: T, pub span: Span }
pub(crate) struct AstAutomaton {
    pub kind: Spanned<AutomatonKind>,
    pub name: Spanned<String>,
    pub items: Vec<AstItem>,
    pub span: Span,
}
pub(crate) enum AstItem {
    Alphabet { symbols: Vec<Spanned<String>>, span: Span },
    Initial { state: Spanned<String>, span: Span },
    Final { states: Vec<Spanned<String>>, span: Span },
    State(AstState),
}
pub(crate) struct AstState {
    pub name: Spanned<String>,
    pub transitions: Vec<AstTransition>,
    pub span: Span,
}
pub(crate) struct AstTransition {
    pub input: Spanned<AstInput>,
    pub destinations: Vec<Spanned<String>>,
    pub span: Span,
}
pub(crate) enum AstInput { Symbol(String), Epsilon }
```

- [ ] **Step 4: Implement parser success paths**

Create a cursor-based `Parser<'a>` with `current`, `advance`, `skip_newlines`, `expect`, and `error`. Parse the grammar from the spec. An item ends at `Newline`, `RightBrace`, a declaration keyword, or `Identifier` followed by `Colon`; newlines remain optional between items. Require commas inside destination lists and transition lists.

- [ ] **Step 5: Implement recovery and EOF checks**

After a syntax error, advance until `Newline`, `RightBrace`, or EOF, then resume at the next item. After the closing `}`, skip newlines and require EOF so a second automaton produces `SYN001`. Use `SYN002` when `}` or `]` is not closed.

- [ ] **Step 6: Run parser and complete library tests**

Run: `cargo test -p salmon-lang parser::tests && cargo test -p salmon-lang`

Expected: all tests pass.

- [ ] **Step 7: Commit the parser**

```bash
git add crates/salmon-lang/src
git commit -m "feat: parse Salmon definitions into a spanned AST"
```

### Task 4: Common semantic analysis and normalization

**Files:**
- Create: `crates/salmon-lang/src/semantic.rs`
- Modify: `crates/salmon-lang/src/lib.rs`

**Interfaces:**
- Consumes: `AstAutomaton` and public model types.
- Produces internally: `analyze(ast: AstAutomaton) -> Result<Automaton, Vec<Diagnostic>>`.
- Guarantees: deterministic first-appearance order for states, alphabet, finals, and transitions.

- [ ] **Step 1: Write failing tests for common rules**

```rust
#[test]
fn infers_alphabet_and_state_order() {
    let automaton = analyze_source("nfa x { initial: q0 final: q2 q0: b=q2, a=q1 }").unwrap();
    assert_eq!(automaton.alphabet, vec![Symbol("b".into()), Symbol("a".into())]);
    assert_eq!(automaton.states, vec![StateId("q0".into()), StateId("q2".into()), StateId("q1".into())]);
}

#[test]
fn reports_missing_initial_and_final_together() {
    let errors = analyze_source("nfa x { q0: a=q1 }").unwrap_err();
    assert_eq!(errors.iter().map(|e| e.code).collect::<Vec<_>>(), vec!["SEM001", "SEM001"]);
}

#[test]
fn rejects_duplicate_declarations_destinations_and_transitions() {
    let source = "nfa x { initial: q0 initial: q1 final: q2 q0: a=[q1,q1], a=q2 }";
    let errors = analyze_source(source).unwrap_err();
    assert!(errors.iter().all(|e| e.code == "SEM002"));
    assert_eq!(errors.len(), 3);
}

#[test]
fn checks_an_explicit_alphabet() {
    let errors = analyze_source("nfa x { alphabet: a initial: q0 final: q1 q0: b=q1 }").unwrap_err();
    assert_eq!(errors[0].code, "SEM003");
}

#[test]
fn rejects_duplicate_explicit_alphabet_symbols() {
    let errors = analyze_source(
        "nfa x { alphabet:a,a initial:q0 final:q1 q0:a=q1 }",
    ).unwrap_err();
    assert!(errors.iter().any(|error| error.code == "SEM002"));
}
```

The test helper calls `lex`, `parse_tokens`, then `analyze`, merging errors exactly as the public API will in Task 6.

```rust
fn analyze_source(source: &str) -> Result<Automaton, Vec<Diagnostic>> {
    let (tokens, lex_errors) = lex(source);
    assert!(lex_errors.is_empty(), "semantic test input must lex");
    let ast = parse_tokens(&tokens).expect("semantic test input must parse");
    analyze(ast)
}
```

- [ ] **Step 2: Run semantic tests and verify they fail**

Run: `cargo test -p salmon-lang semantic::tests`

Expected: compilation fails because `analyze` does not exist.

- [ ] **Step 3: Collect declarations and first-appearance order**

Walk AST items once. Use `Vec` plus `HashSet<String>` to preserve order while detecting duplicates. Add a state when it first appears in `initial`, `final`, an origin, or a destination. Add inferred symbols when they first appear in non-epsilon transitions.

- [ ] **Step 4: Implement common validation**

Emit:

```text
SEM001 missing initial declaration
SEM001 missing final declaration
SEM002 duplicate initial declaration
SEM002 duplicate final declaration
SEM002 duplicate alphabet symbol `<symbol>`
SEM002 duplicate destination `<state>`
SEM002 duplicate transition from `<state>` on `<input>`
SEM003 symbol `<symbol>` is not in the declared alphabet
```

Reject `eps` in explicit alphabets during parsing as `SYN001`, because it is a reserved token and cannot satisfy `symbol`.

- [ ] **Step 5: Build the normalized model only when no errors exist**

Convert strings to `StateId` and `Symbol`; convert `AstInput::Epsilon` to `None`. Preserve first-appearance ordering and AST transition order. Return every collected diagnostic together rather than exiting after the first semantic failure.

- [ ] **Step 6: Run semantic tests and lint**

Run: `cargo test -p salmon-lang semantic::tests && cargo clippy -p salmon-lang --all-targets -- -D warnings`

Expected: all tests pass and Clippy emits no warnings.

- [ ] **Step 7: Commit common semantic analysis**

```bash
git add crates/salmon-lang/src
git commit -m "feat: validate and normalize Salmon automata"
```

### Task 5: DFA, NFA, and ε-NFA type rules

**Files:**
- Modify: `crates/salmon-lang/src/semantic.rs`

**Interfaces:**
- Extends: `analyze` with kind-specific checks after common collection.
- Guarantees: successful `Dfa` values are deterministic, epsilon-free, and complete.

- [ ] **Step 1: Add a failing type-rule test matrix**

```rust
#[test]
fn enforces_kind_specific_rules() {
    let cases = [
        ("dfa x { initial:q0 final:q1 q0:eps=q1 }", "SEM004"),
        ("nfa x { initial:q0 final:q1 q0:eps=q1 }", "SEM004"),
        ("dfa x { initial:q0 final:q1 q0:a=[q0,q1] q1:a=q1 }", "SEM005"),
        ("dfa x { initial:q0 final:q1 q0:a=q1 }", "SEM006"),
    ];
    for (source, code) in cases {
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|error| error.code == code), "missing {code}");
    }
}

#[test]
fn permits_epsilon_only_for_enfa() {
    let automaton = analyze_source("enfa x { initial:q0 final:q1 q0:eps=q1 }").unwrap();
    assert_eq!(automaton.transitions[0].input, None);
}
```

- [ ] **Step 2: Run the new tests and verify they fail**

Run: `cargo test -p salmon-lang semantic::tests`

Expected: at least the DFA/NFA rejection assertions fail.

- [ ] **Step 3: Add epsilon and determinism checks**

For `Dfa` and `Nfa`, emit `SEM004` for every epsilon transition. For `Dfa`, emit `SEM005` for every transition with more than one destination. Do not emit secondary completeness errors for an origin/input pair already rejected as nondeterministic.

- [ ] **Step 4: Add DFA completeness validation**

After collecting all states and alphabet symbols, build a `HashSet<(String, String)>` of valid deterministic transitions. Iterate state-major then alphabet-major and emit `SEM006` for each missing pair. Attach the diagnostic to the state's first occurrence span so line and column remain meaningful.

- [ ] **Step 5: Run the entire language crate suite**

Run: `cargo test -p salmon-lang && cargo clippy -p salmon-lang --all-targets -- -D warnings`

Expected: all tests pass with no warnings.

- [ ] **Step 6: Commit kind-specific validation**

```bash
git add crates/salmon-lang/src/semantic.rs
git commit -m "feat: enforce automaton kind semantics"
```

### Task 6: Public parse API and end-to-end language fixtures

**Files:**
- Modify: `crates/salmon-lang/src/lib.rs`
- Create: `crates/salmon-lang/tests/language.rs`

**Interfaces:**
- Produces: `pub fn parse(source_name: &str, source: &str) -> Result<Automaton, Vec<Diagnostic>>`.
- Re-exports: model types plus `Diagnostic`, `Severity`, `Span`, and `render_diagnostic`.

- [ ] **Step 1: Write public API integration tests for all three kinds**

```rust
use salmon_lang::{parse, AutomatonKind, StateId, Symbol};

#[test]
fn parses_a_complete_dfa() {
    let source = "dfa toggle { initial:q0 final:q1 q0:a=q1 q1:a=q0 }";
    let automaton = parse("toggle.salmon", source).unwrap();
    assert_eq!(automaton.kind, AutomatonKind::Dfa);
    assert_eq!(automaton.alphabet, vec![Symbol("a".into())]);
    assert_eq!(automaton.initial, StateId("q0".into()));
}

#[test]
fn parses_an_nfa_with_a_quoted_symbol() {
    let source = "nfa x { initial:q0 final:q1 q0:\"end value\"=[q0,q1] }";
    assert!(parse("nfa.salmon", source).is_ok());
}

#[test]
fn parses_an_enfa() {
    let source = "enfa x { initial:q0 final:q1 q0:eps=q1 }";
    assert!(parse("enfa.salmon", source).is_ok());
}

#[test]
fn parses_the_explicit_alphabet_example_from_the_spec() {
    let source = r#"nfa words {
        alphabet: a, b, "end"
        initial: start
        final: left, right
        start: a=[left, right], "end"=right
    }"#;
    let automaton = parse("words.salmon", source).unwrap();
    assert_eq!(automaton.finals, vec![StateId("left".into()), StateId("right".into())]);
    assert_eq!(automaton.alphabet.last(), Some(&Symbol("end".into())));
}

#[test]
fn returns_lexical_and_syntax_errors_without_running_semantics() {
    let errors = parse("bad.salmon", "dfa x { @\ninitial q0\n}").unwrap_err();
    assert!(errors.iter().any(|e| e.code == "SYN001"));
    assert!(!errors.iter().any(|e| e.code.starts_with("SEM")));
}
```

- [ ] **Step 2: Run integration tests and verify they fail**

Run: `cargo test -p salmon-lang --test language`

Expected: compilation fails because `parse` and the public re-exports are absent.

- [ ] **Step 3: Wire the pipeline in `lib.rs`**

```rust
pub fn parse(source_name: &str, source: &str) -> Result<Automaton, Vec<Diagnostic>> {
    let (tokens, mut diagnostics) = lexer::lex(source);
    match parser::parse_tokens(&tokens) {
        Ok(ast) if diagnostics.is_empty() => semantic::analyze(ast),
        Ok(_) => Err(diagnostics),
        Err(mut parser_diagnostics) => {
            diagnostics.append(&mut parser_diagnostics);
            let _ = source_name;
            Err(diagnostics)
        }
    }
}
```

Keep `source_name` in the signature for API stability; diagnostics store spans, while rendering supplies the name. Do not run semantic analysis if lexing or parsing produced errors.

- [ ] **Step 4: Run integration tests and full crate checks**

Run: `cargo test -p salmon-lang && cargo fmt --all -- --check && cargo clippy -p salmon-lang --all-targets -- -D warnings`

Expected: all tests and checks pass. If Clippy flags the intentionally unused `source_name`, rename it to `_source_name` without changing the public signature's argument position.

- [ ] **Step 5: Commit the public library API**

```bash
git add crates/salmon-lang
git commit -m "feat: expose end-to-end Salmon parse API"
```

### Task 7: `salmon check` command

**Files:**
- Modify: `crates/salmon-cli/Cargo.toml`
- Modify: `crates/salmon-cli/src/main.rs`
- Create: `crates/salmon-cli/tests/check.rs`
- Create: `crates/salmon-cli/tests/fixtures/valid.salmon`
- Create: `crates/salmon-cli/tests/fixtures/invalid.salmon`

**Interfaces:**
- Consumes: `salmon_lang::parse` and `render_diagnostic`.
- Produces: `salmon check <archivo>` with exit codes `0`, `1`, and `2`.

- [ ] **Step 1: Finalize the CLI manifest**

```toml
[package]
name = "salmon-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[[bin]]
name = "salmon"
path = "src/main.rs"

[dependencies]
salmon-lang = { path = "../salmon-lang" }
```

- [ ] **Step 2: Add fixtures and failing process tests**

Set `valid.salmon` to the complete `ends_with_ab` DFA from the spec. Set `invalid.salmon` to:

```salmon
dfa broken {
    initial: q0
    final: q1
    q0: eps=q1
}
```

Add:

```rust
use std::path::PathBuf;
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

#[test]
fn valid_file_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .args(["check", fixture("valid.salmon").to_str().unwrap()])
        .output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn invalid_definition_exits_one_with_a_positioned_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .args(["check", fixture("invalid.salmon").to_str().unwrap()])
        .output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("invalid.salmon:4:9: error[SEM004]"));
}

#[test]
fn usage_and_io_failures_exit_two() {
    let no_args = Command::new(env!("CARGO_BIN_EXE_salmon")).output().unwrap();
    assert_eq!(no_args.status.code(), Some(2));
    let missing = Command::new(env!("CARGO_BIN_EXE_salmon"))
        .args(["check", "does-not-exist.salmon"]).output().unwrap();
    assert_eq!(missing.status.code(), Some(2));
}
```

- [ ] **Step 3: Run CLI tests and verify they fail**

Run: `cargo test -p salmon-cli --test check`

Expected: tests fail because the stub CLI does not implement `check` or exit codes.

- [ ] **Step 4: Implement argument parsing and checking**

Use `std::env::args_os`; accept exactly `salmon check <path>`. Read with `std::fs::read_to_string`. On success return `ExitCode::SUCCESS`; on language diagnostics render each to stderr and return `ExitCode::from(1)`; on invalid usage or I/O failure print one concise error and return `ExitCode::from(2)`.

The usage string must be:

```text
usage: salmon check <file.salmon>
```

- [ ] **Step 5: Run CLI tests and manual smoke checks**

Run: `cargo test -p salmon-cli --test check && cargo run -q -p salmon-cli -- check crates/salmon-cli/tests/fixtures/valid.salmon`

Expected: all tests pass; the valid smoke check prints nothing and exits `0`.

- [ ] **Step 6: Commit the CLI**

```bash
git add crates/salmon-cli
git commit -m "feat: add Salmon validation CLI"
```

### Task 8: Documentation and release-quality verification

**Files:**
- Create: `README.md`
- Modify: `crates/salmon-lang/tests/language.rs`

**Interfaces:**
- Documents: supported kinds, compact syntax, optional alphabet, `eps`, installation, and `salmon check`.
- Verifies: every acceptance criterion from the design specification.

- [ ] **Step 1: Write a failing documentation example test**

Add to `crates/salmon-lang/tests/language.rs`:

```rust
#[test]
fn readme_example_stays_valid() {
    let source = include_str!("../../../README.md");
    let start = source.find("```salmon\n").unwrap() + "```salmon\n".len();
    let end = source[start..].find("\n```").unwrap() + start;
    parse("README.md", &source[start..end]).unwrap();
}
```

- [ ] **Step 2: Run the documentation test and verify it fails**

Run: `cargo test -p salmon-lang --test language readme_example_stays_valid`

Expected: test fails because `README.md` does not exist or has no Salmon code block.

- [ ] **Step 3: Create the README**

Include the project purpose, the approved `ends_with_ab` example as the first `salmon` block, supported `dfa`/`nfa`/`enfa` kinds, optional `alphabet`, `eps`, `cargo build`, and this usage:

```bash
cargo run -p salmon-cli -- check path/to/machine.salmon
```

State explicitly that simulation, transformations, regex conversion, and UI are future work.

- [ ] **Step 4: Run every automated quality gate**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Expected: every command exits `0`; all unit, integration, CLI, and README tests pass.

- [ ] **Step 5: Exercise all CLI exit paths**

Run:

```bash
cargo run -q -p salmon-cli -- check crates/salmon-cli/tests/fixtures/valid.salmon
cargo run -q -p salmon-cli -- check crates/salmon-cli/tests/fixtures/invalid.salmon
cargo run -q -p salmon-cli -- check does-not-exist.salmon
```

Expected: the first command exits `0`; the second prints a positioned `SEM004` and exits `1`; the third prints an I/O error and exits `2`. Run separately when checking shell exit codes so later commands do not hide earlier failures.

- [ ] **Step 6: Confirm the public model remains parser-independent**

Run: `rg -n 'crate::(ast|lexer|parser|semantic)|super::(ast|lexer|parser|semantic)' crates/salmon-lang/src/model.rs`

Expected: no matches.

- [ ] **Step 7: Commit documentation and final polish**

```bash
git add README.md crates
git commit -m "docs: add Salmon language quick start"
```

## Completion Evidence

Before declaring the MVP complete, record the output of:

```bash
git status --short
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

The work is complete only when the worktree contains no unintended changes and every quality command exits `0`.
