use std::process::Command;

fn salmon() -> Command {
    Command::new(env!("CARGO_BIN_EXE_salmon"))
}

fn dfa() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/dfa.salmon")
}

fn nfa() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/nfa.salmon")
}

fn enfa() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/enfa.salmon")
}

fn invalid_fixture() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/invalid.salmon")
}

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
    assert_eq!(malformed.stdout, b"");
    assert_eq!(malformed.stderr, b"ERROR: unclosed quoted symbol\n");

    let invalid = salmon()
        .args(["run", invalid_fixture(), "a"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(invalid.stdout.is_empty());
}

#[test]
fn one_shot_runs_nfa_examples() {
    let accepted = salmon().args(["run", nfa(), "ab"]).output().unwrap();
    assert_eq!(accepted.status.code(), Some(0));
    assert_eq!(accepted.stdout, b"OK\n");

    let rejected = salmon().args(["run", nfa(), "aaa"]).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(rejected.stdout, b"NOT\n");
}

#[test]
fn one_shot_runs_enfa_examples() {
    let accepted = salmon().args(["run", enfa(), "aa"]).output().unwrap();
    assert_eq!(accepted.status.code(), Some(0));
    assert_eq!(accepted.stdout, b"OK\n");

    let rejected = salmon().args(["run", enfa(), ""]).output().unwrap();
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(rejected.stdout, b"NOT\n");
}

#[cfg(unix)]
#[test]
fn one_shot_rejects_a_non_unicode_input_argument() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let output = salmon()
        .arg("run")
        .arg(dfa())
        .arg(OsString::from_vec(b"\xff".to_vec()))
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"ERROR: input must be valid Unicode\n");
}
