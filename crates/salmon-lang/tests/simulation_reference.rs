use std::collections::HashSet;

use salmon_lang::{Automaton, AutomatonKind, StateId, Symbol, Transition, accepts};

#[test]
fn repository_examples_match_their_language_definitions() {
    let examples = [
        (include_str!("../../../examples/dfa.salmon"), "01"),
        (include_str!("../../../examples/nfa.salmon"), "ab"),
        (include_str!("../../../examples/enfa.salmon"), "ab"),
    ];
    for (index, (source, alphabet)) in examples.iter().enumerate() {
        let machine = salmon_lang::parse("example.salmon", source).unwrap();
        let symbols: Vec<_> = alphabet.chars().collect();
        for length in 0..=7 {
            for bits in 0..(1 << length) {
                let word: String = (0..length).map(|i| symbols[(bits >> i) & 1]).collect();
                let expected = match index {
                    0 => word.chars().filter(|c| *c == '1').count() % 2 == 0,
                    1 => word.contains("ab"),
                    _ => !word.is_empty() && word.chars().all(|c| c == 'a'),
                };
                let input: Vec<_> = word.chars().map(|c| Symbol(c.to_string())).collect();
                assert_eq!(
                    accepts(&machine, &input),
                    expected,
                    "{}: {word:?}",
                    machine.name
                );
            }
        }
    }
}

// An independent evaluator explores individual (state, input position) paths.
// Unlike the production simulator, it never constructs epsilon-closure sets.
fn accepts_by_paths(machine: &Automaton, word: &[Symbol]) -> bool {
    let mut pending = vec![(machine.initial.clone(), 0)];
    let mut visited = HashSet::new();
    while let Some((state, position)) = pending.pop() {
        if !visited.insert((state.clone(), position)) {
            continue;
        }
        if position == word.len() && machine.finals.contains(&state) {
            return true;
        }
        for edge in machine.transitions.iter().filter(|edge| edge.from == state) {
            let next_position = match &edge.input {
                None => position,
                Some(symbol) if word.get(position) == Some(symbol) => position + 1,
                Some(_) => continue,
            };
            pending.extend(edge.to.iter().cloned().map(|state| (state, next_position)));
        }
    }
    false
}

#[test]
fn generated_automata_agree_with_an_independent_path_evaluator() {
    let states: Vec<_> = (0..3).map(|i| StateId(format!("q{i}"))).collect();
    let alphabet = vec![Symbol("a".into()), Symbol("b".into())];
    let mut seed = 0x5a17_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize
    };
    for kind in [AutomatonKind::Dfa, AutomatonKind::Nfa, AutomatonKind::Enfa] {
        for case in 0..128 {
            let mut transitions = Vec::new();
            for from in &states {
                for input in alphabet
                    .iter()
                    .cloned()
                    .map(Some)
                    .chain((kind == AutomatonKind::Enfa).then_some(None))
                {
                    let destinations = if kind == AutomatonKind::Dfa {
                        vec![states[next() % states.len()].clone()]
                    } else {
                        let mask = next() % 8;
                        states
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| mask & (1 << i) != 0)
                            .map(|(_, state)| state.clone())
                            .collect()
                    };
                    transitions.push(Transition {
                        from: from.clone(),
                        input,
                        to: destinations,
                    });
                }
            }
            let machine = Automaton {
                name: format!("generated-{case}"),
                kind,
                alphabet: alphabet.clone(),
                states: states.clone(),
                initial: states[next() % 3].clone(),
                finals: vec![states[next() % 3].clone()],
                transitions,
            };
            for length in 0..=5 {
                for bits in 0..(1 << length) {
                    let word: Vec<_> = (0..length)
                        .map(|i| alphabet[(bits >> i) & 1].clone())
                        .collect();
                    assert_eq!(
                        accepts(&machine, &word),
                        accepts_by_paths(&machine, &word),
                        "kind={kind:?}, case={case}, word={word:?}"
                    );
                }
            }
        }
    }
}
