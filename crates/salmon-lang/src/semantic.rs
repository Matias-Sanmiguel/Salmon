use std::collections::{HashMap, HashSet};

use crate::ast::{AstAutomaton, AstInput, AstItem, Spanned};
use crate::diagnostic::{Diagnostic, Span};
use crate::model::{Automaton, AutomatonKind, StateId, Symbol, Transition};

pub(crate) fn analyze(mut ast: AstAutomaton) -> Result<Automaton, Vec<Diagnostic>> {
    let mut analysis = Analysis::default();

    for item in std::mem::take(&mut ast.items) {
        match item {
            AstItem::Alphabet { symbols, .. } => analysis.collect_alphabet(symbols),
            AstItem::Initial { state, span } => analysis.collect_initial(state, span),
            AstItem::Final { states, span } => analysis.collect_finals(states, span),
            AstItem::State(state) => analysis.collect_state(state.name, state.transitions),
        }
    }

    analysis.finish(ast)
}

#[derive(Default)]
struct Analysis {
    alphabet: Vec<String>,
    alphabet_set: HashSet<String>,
    explicit_alphabet: bool,
    states: Vec<String>,
    state_set: HashSet<String>,
    state_spans: HashMap<String, Span>,
    initial: Option<String>,
    initial_count: usize,
    finals: Vec<String>,
    final_set: HashSet<String>,
    final_count: usize,
    used_symbols: Vec<Spanned<String>>,
    transitions: Vec<RawTransition>,
    transition_set: HashSet<(String, Option<String>)>,
    diagnostics: Vec<Diagnostic>,
}

struct RawTransition {
    from: String,
    input: Option<String>,
    destinations: Vec<String>,
    span: Span,
}

impl Analysis {
    fn collect_alphabet(&mut self, symbols: Vec<Spanned<String>>) {
        self.explicit_alphabet = true;

        for symbol in symbols {
            if !self.alphabet_set.insert(symbol.value.clone()) {
                self.diagnostics.push(Diagnostic::error(
                    "SEM002",
                    format!("duplicate alphabet symbol `{}`", symbol.value),
                    symbol.span,
                ));
            } else {
                self.alphabet.push(symbol.value);
            }
        }
    }

    fn collect_initial(&mut self, state: Spanned<String>, span: Span) {
        self.add_state(&state);
        self.initial_count += 1;
        if self.initial_count == 1 {
            self.initial = Some(state.value);
        } else {
            self.diagnostics.push(Diagnostic::error(
                "SEM002",
                "duplicate initial declaration",
                span,
            ));
        }
    }

    fn collect_finals(&mut self, states: Vec<Spanned<String>>, span: Span) {
        self.final_count += 1;
        if self.final_count > 1 {
            self.diagnostics.push(Diagnostic::error(
                "SEM002",
                "duplicate final declaration",
                span,
            ));
        }

        for state in states {
            self.add_state(&state);
            if self.final_set.insert(state.value.clone()) {
                self.finals.push(state.value);
            }
        }
    }

    fn collect_state(
        &mut self,
        state: Spanned<String>,
        transitions: Vec<crate::ast::AstTransition>,
    ) {
        self.add_state(&state);

        for transition in transitions {
            let input = match transition.input.value {
                AstInput::Symbol(symbol) => Some(symbol),
                AstInput::Epsilon => None,
            };
            let input_label = input.as_deref().unwrap_or("eps");
            let key = (state.value.clone(), input.clone());
            if !self.transition_set.insert(key) {
                self.diagnostics.push(Diagnostic::error(
                    "SEM002",
                    format!(
                        "duplicate transition from `{}` on `{input_label}`",
                        state.value
                    ),
                    transition.span,
                ));
            }

            if let Some(symbol) = &input {
                self.used_symbols.push(Spanned {
                    value: symbol.clone(),
                    span: transition.input.span,
                });
            }

            let mut destinations = Vec::with_capacity(transition.destinations.len());
            let mut destination_set = HashSet::new();
            for destination in transition.destinations {
                self.add_state(&destination);
                if !destination_set.insert(destination.value.clone()) {
                    self.diagnostics.push(Diagnostic::error(
                        "SEM002",
                        format!("duplicate destination `{}`", destination.value),
                        destination.span,
                    ));
                }
                destinations.push(destination.value);
            }

            self.transitions.push(RawTransition {
                from: state.value.clone(),
                input,
                destinations,
                span: transition.span,
            });
        }
    }

    fn add_state(&mut self, state: &Spanned<String>) {
        if self.state_set.insert(state.value.clone()) {
            self.state_spans.insert(state.value.clone(), state.span);
            self.states.push(state.value.clone());
        }
    }

    fn finish(mut self, ast: AstAutomaton) -> Result<Automaton, Vec<Diagnostic>> {
        if self.explicit_alphabet {
            for symbol in &self.used_symbols {
                if !self.alphabet_set.contains(&symbol.value) {
                    self.diagnostics.push(Diagnostic::error(
                        "SEM003",
                        format!("symbol `{}` is not in the declared alphabet", symbol.value),
                        symbol.span,
                    ));
                }
            }
        } else {
            for symbol in &self.used_symbols {
                if self.alphabet_set.insert(symbol.value.clone()) {
                    self.alphabet.push(symbol.value.clone());
                }
            }
        }

        self.validate_kind(ast.kind.value);

        if self.initial_count == 0 {
            self.diagnostics.push(Diagnostic::error(
                "SEM001",
                "missing initial declaration",
                ast.span,
            ));
        }
        if self.final_count == 0 {
            self.diagnostics.push(Diagnostic::error(
                "SEM001",
                "missing final declaration",
                ast.span,
            ));
        }

        if !self.diagnostics.is_empty() {
            return Err(self.diagnostics);
        }

        Ok(Automaton {
            name: ast.name.value,
            kind: ast.kind.value,
            alphabet: self.alphabet.into_iter().map(Symbol).collect(),
            states: self.states.into_iter().map(StateId).collect(),
            initial: StateId(
                self.initial
                    .expect("a successful analysis has an initial state"),
            ),
            finals: self.finals.into_iter().map(StateId).collect(),
            transitions: self
                .transitions
                .into_iter()
                .map(|transition| Transition {
                    from: StateId(transition.from),
                    input: transition.input.map(Symbol),
                    to: transition.destinations.into_iter().map(StateId).collect(),
                })
                .collect(),
        })
    }

    fn validate_kind(&mut self, kind: AutomatonKind) {
        if matches!(kind, AutomatonKind::Dfa | AutomatonKind::Nfa) {
            for transition in &self.transitions {
                if transition.input.is_none() {
                    self.diagnostics.push(Diagnostic::error(
                        "SEM004",
                        "epsilon transitions are only allowed in enfa",
                        transition.span,
                    ));
                }
            }
        }

        if kind != AutomatonKind::Dfa {
            return;
        }

        let mut deterministic = HashSet::new();
        let mut nondeterministic = HashSet::new();
        for transition in &self.transitions {
            if transition.destinations.len() > 1 {
                self.diagnostics.push(Diagnostic::error(
                    "SEM005",
                    "dfa transitions must have exactly one destination",
                    transition.span,
                ));
            }

            let Some(input) = &transition.input else {
                continue;
            };
            let key = (transition.from.clone(), input.clone());
            if transition.destinations.len() > 1 {
                nondeterministic.insert(key);
            } else {
                deterministic.insert(key);
            }
        }

        for state in &self.states {
            for symbol in &self.alphabet {
                let key = (state.clone(), symbol.clone());
                if !deterministic.contains(&key) && !nondeterministic.contains(&key) {
                    self.diagnostics.push(Diagnostic::error(
                        "SEM006",
                        format!("missing dfa transition from `{state}` on `{symbol}`"),
                        self.state_spans[state],
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::analyze;
    use crate::diagnostic::Diagnostic;
    use crate::lexer::lex;
    use crate::model::{Automaton, StateId, Symbol};
    use crate::parser::parse_tokens;

    fn analyze_source(source: &str) -> Result<Automaton, Vec<Diagnostic>> {
        let (tokens, lex_errors) = lex(source);
        assert!(lex_errors.is_empty(), "semantic test input must lex");
        let ast = parse_tokens(&tokens).expect("semantic test input must parse");
        analyze(ast)
    }

    #[test]
    fn infers_alphabet_and_state_order() {
        let automaton = analyze_source("nfa x { initial: q0 final: q2 q0: b=q2, a=q1 }").unwrap();
        assert_eq!(
            automaton.alphabet,
            vec![Symbol("b".into()), Symbol("a".into())]
        );
        assert_eq!(
            automaton.states,
            vec![
                StateId("q0".into()),
                StateId("q2".into()),
                StateId("q1".into())
            ]
        );
    }

    #[test]
    fn reports_missing_initial_and_final_together() {
        let errors = analyze_source("nfa x { q0: a=q1 }").unwrap_err();
        assert_eq!(
            errors.iter().map(|error| error.code).collect::<Vec<_>>(),
            vec!["SEM001", "SEM001"]
        );
    }

    #[test]
    fn rejects_duplicate_declarations_destinations_and_transitions() {
        let source = "nfa x { initial: q0 initial: q1 final: q2 q0: a=[q1,q1], a=q2 }";
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().all(|error| error.code == "SEM002"));
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn checks_an_explicit_alphabet() {
        let errors =
            analyze_source("nfa x { alphabet: a initial: q0 final: q1 q0: b=q1 }").unwrap_err();
        assert_eq!(errors[0].code, "SEM003");
    }

    #[test]
    fn validates_transitions_against_an_alphabet_declared_later() {
        let errors =
            analyze_source("nfa x { initial: q0 final: q1 q0: b=q1 alphabet: a }").unwrap_err();
        assert_eq!(errors[0].code, "SEM003");
    }

    #[test]
    fn rejects_duplicate_explicit_alphabet_symbols() {
        let errors =
            analyze_source("nfa x { alphabet:a,a initial:q0 final:q1 q0:a=q1 }").unwrap_err();
        assert!(errors.iter().any(|error| error.code == "SEM002"));
    }

    #[test]
    fn enforces_kind_specific_rules() {
        let cases = [
            ("dfa x { initial:q0 final:q1 q0:eps=q1 }", "SEM004"),
            ("nfa x { initial:q0 final:q1 q0:eps=q1 }", "SEM004"),
            (
                "dfa x { initial:q0 final:q1 q0:a=[q0,q1] q1:a=q1 }",
                "SEM005",
            ),
            ("dfa x { initial:q0 final:q1 q0:a=q1 }", "SEM006"),
        ];
        for (source, code) in cases {
            let errors = analyze_source(source).unwrap_err();
            assert!(
                errors.iter().any(|error| error.code == code),
                "missing {code}"
            );
        }
    }

    #[test]
    fn permits_epsilon_only_for_enfa() {
        let automaton = analyze_source("enfa x { initial:q0 final:q1 q0:eps=q1 }").unwrap();
        assert_eq!(automaton.transitions[0].input, None);
    }

    #[test]
    fn reports_nondeterministic_epsilon_transitions_in_dfas() {
        let errors = analyze_source("dfa x { initial:q0 final:q1 q0:eps=[q0,q1] }").unwrap_err();
        assert!(errors.iter().any(|error| error.code == "SEM004"));
        assert!(errors.iter().any(|error| error.code == "SEM005"));
    }
}
