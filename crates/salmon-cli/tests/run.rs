use std::io::Write;
use std::process::{Command, Stdio};

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
fn interactive_session_prints_a_prompt_for_each_word() {
    let mut child = salmon()
        .args(["run", dfa()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"11\n1\n\n:quit\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        b"Salmon \xe2\x80\x94 even_ones\n> OK\n> NOT\n> OK\n> "
    );
}

#[test]
fn interactive_session_recovers_after_a_malformed_word() {
    let mut child = salmon()
        .args(["run", dfa()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"\"unterminated\n11\n:quit\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ERROR: unclosed quoted symbol\n"));
    assert!(stdout.contains("> OK\n"));
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

#[test]
fn example_matrix() {
    let cases = [
        (dfa(), "", 0, "OK\n"),
        (dfa(), "11", 0, "OK\n"),
        (dfa(), "1", 1, "NOT\n"),
        (nfa(), "ab", 0, "OK\n"),
        (nfa(), "baba", 0, "OK\n"),
        (nfa(), "aaa", 1, "NOT\n"),
        (enfa(), "a", 0, "OK\n"),
        (enfa(), "aa", 0, "OK\n"),
        (enfa(), "", 1, "NOT\n"),
    ];

    for (path, input, expected_status, expected_stdout) in cases {
        let output = salmon().args(["run", path, input]).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected_status),
            "unexpected exit status for {path} with input {input:?}"
        );
        assert_eq!(
            output.stdout,
            expected_stdout.as_bytes(),
            "unexpected stdout for {path} with input {input:?}"
        );
        assert!(
            output.stderr.is_empty(),
            "unexpected stderr for {path} with input {input:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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
