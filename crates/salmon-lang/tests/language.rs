use salmon_lang::{AutomatonKind, StateId, Symbol, parse};

#[test]
fn readme_example_stays_valid() {
    let source = include_str!("../../../README.md");
    let start = source.find("```salmon\n").unwrap() + "```salmon\n".len();
    let end = source[start..].find("\n```").unwrap() + start;
    parse("README.md", &source[start..end]).unwrap();
}

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
    assert_eq!(
        automaton.finals,
        vec![StateId("left".into()), StateId("right".into())]
    );
    assert_eq!(automaton.alphabet.last(), Some(&Symbol("end".into())));
}

#[test]
fn returns_lexical_and_syntax_errors_without_running_semantics() {
    let errors = parse("bad.salmon", "dfa x { @\ninitial q0\n}").unwrap_err();
    assert!(errors.iter().any(|error| error.code == "SYN001"));
    assert!(!errors.iter().any(|error| error.code.starts_with("SEM")));
}
