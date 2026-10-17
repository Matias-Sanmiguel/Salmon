use salmon_lang::{AutomatonKind, StateId, Symbol, Transition, parse};

#[test]
fn rejects_repeated_alphabet_declarations_even_with_disjoint_symbols() {
    let source = "nfa x { alphabet:a alphabet:b initial:q0 final:q1 q0:a=q1 }";
    let errors = parse("duplicate.salmon", source).unwrap_err();
    assert!(
        errors.iter().any(
            |error| error.code == "SEM002" && error.message == "duplicate alphabet declaration"
        )
    );
}

#[test]
fn readme_example_stays_valid() {
    let source = include_str!("../../../README.md");
    let start = source.find("```salmon\n").unwrap() + "```salmon\n".len();
    let end = source[start..].find("\n```").unwrap() + start;
    parse("README.md", &source[start..end]).unwrap();
}

#[test]
fn parses_the_exact_dfa_example_from_the_spec() {
    let source = r#"dfa ends_with_ab {
    initial: q0
    final: q2

    q0: a=q1, b=q0
    q1: a=q1, b=q2
    q2: a=q1, b=q0
}"#;

    let automaton = parse("ends_with_ab.salmon", source).unwrap();

    assert_eq!(automaton.name, "ends_with_ab");
    assert_eq!(automaton.kind, AutomatonKind::Dfa);
    assert_eq!(
        automaton.alphabet,
        vec![Symbol("a".into()), Symbol("b".into())]
    );
    assert_eq!(
        automaton.states,
        vec![
            StateId("q0".into()),
            StateId("q2".into()),
            StateId("q1".into())
        ]
    );
    assert_eq!(automaton.initial, StateId("q0".into()));
    assert_eq!(automaton.finals, vec![StateId("q2".into())]);
    assert_eq!(
        automaton.transitions,
        vec![
            Transition {
                from: StateId("q0".into()),
                input: Some(Symbol("a".into())),
                to: vec![StateId("q1".into())],
            },
            Transition {
                from: StateId("q0".into()),
                input: Some(Symbol("b".into())),
                to: vec![StateId("q0".into())],
            },
            Transition {
                from: StateId("q1".into()),
                input: Some(Symbol("a".into())),
                to: vec![StateId("q1".into())],
            },
            Transition {
                from: StateId("q1".into()),
                input: Some(Symbol("b".into())),
                to: vec![StateId("q2".into())],
            },
            Transition {
                from: StateId("q2".into()),
                input: Some(Symbol("a".into())),
                to: vec![StateId("q1".into())],
            },
            Transition {
                from: StateId("q2".into()),
                input: Some(Symbol("b".into())),
                to: vec![StateId("q0".into())],
            },
        ]
    );
}

#[test]
fn parses_the_exact_enfa_example_from_the_spec() {
    let source = r#"enfa example {
    initial: q0
    final: q2

    q0: a=[q0, q1], eps=q2
    q1: b=q2
}"#;

    let automaton = parse("example.salmon", source).unwrap();

    assert_eq!(automaton.name, "example");
    assert_eq!(automaton.kind, AutomatonKind::Enfa);
    assert_eq!(
        automaton.alphabet,
        vec![Symbol("a".into()), Symbol("b".into())]
    );
    assert_eq!(
        automaton.states,
        vec![
            StateId("q0".into()),
            StateId("q2".into()),
            StateId("q1".into())
        ]
    );
    assert_eq!(automaton.initial, StateId("q0".into()));
    assert_eq!(automaton.finals, vec![StateId("q2".into())]);
    assert_eq!(
        automaton.transitions,
        vec![
            Transition {
                from: StateId("q0".into()),
                input: Some(Symbol("a".into())),
                to: vec![StateId("q0".into()), StateId("q1".into())],
            },
            Transition {
                from: StateId("q0".into()),
                input: None,
                to: vec![StateId("q2".into())],
            },
            Transition {
                from: StateId("q1".into()),
                input: Some(Symbol("b".into())),
                to: vec![StateId("q2".into())],
            },
        ]
    );
}

#[test]
fn parses_the_exact_explicit_alphabet_example_from_the_spec() {
    let source = r#"nfa words {
    alphabet: a, b, "end"
    initial: start
    final: left, right

    start: a=[left, right], "end"=right
}"#;

    let automaton = parse("words.salmon", source).unwrap();

    assert_eq!(automaton.name, "words");
    assert_eq!(automaton.kind, AutomatonKind::Nfa);
    assert_eq!(
        automaton.alphabet,
        vec![Symbol("a".into()), Symbol("b".into()), Symbol("end".into())]
    );
    assert_eq!(
        automaton.states,
        vec![
            StateId("start".into()),
            StateId("left".into()),
            StateId("right".into())
        ]
    );
    assert_eq!(automaton.initial, StateId("start".into()));
    assert_eq!(
        automaton.finals,
        vec![StateId("left".into()), StateId("right".into())]
    );
    assert_eq!(
        automaton.transitions,
        vec![
            Transition {
                from: StateId("start".into()),
                input: Some(Symbol("a".into())),
                to: vec![StateId("left".into()), StateId("right".into())],
            },
            Transition {
                from: StateId("start".into()),
                input: Some(Symbol("end".into())),
                to: vec![StateId("right".into())],
            },
        ]
    );
}

#[test]
fn rejects_a_duplicate_final_declaration() {
    let source = "nfa duplicate_final { initial:q0 final:q1 final:q2 q0:a=q1 }";

    let errors = parse("duplicate-final.salmon", source).unwrap_err();

    assert!(
        errors.iter().any(|error| {
            error.code == "SEM002" && error.message == "duplicate final declaration"
        })
    );
}

#[test]
fn rejects_an_empty_destination_list() {
    let source = "nfa empty_destinations { initial:q0 final:q1 q0:a=[] }";

    let errors = parse("empty-destinations.salmon", source).unwrap_err();

    assert!(errors.iter().any(|error| error.code == "SYN001"));
}

#[test]
fn excludes_unquoted_epsilon_from_the_inferred_alphabet() {
    let source = "enfa epsilon { initial:q0 final:q1 q0:eps=q1 q1:a=q1 }";

    let automaton = parse("epsilon.salmon", source).unwrap();

    assert_eq!(automaton.alphabet, vec![Symbol("a".into())]);
}

#[test]
fn rejects_unquoted_epsilon_in_an_explicit_alphabet() {
    let source = "enfa epsilon { alphabet:eps initial:q0 final:q1 q0:eps=q1 }";

    let errors = parse("epsilon-alphabet.salmon", source).unwrap_err();

    assert!(errors.iter().any(|error| error.code == "SYN001"));
}

#[test]
fn returns_lexical_and_syntax_errors_without_running_semantics() {
    let errors = parse("bad.salmon", "dfa x { @\ninitial q0\n}").unwrap_err();
    assert!(errors.iter().any(|error| error.code == "SYN001"));
    assert!(!errors.iter().any(|error| error.code.starts_with("SEM")));
}
