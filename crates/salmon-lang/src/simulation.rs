use std::collections::HashSet;

use crate::{Automaton, StateId, Symbol};

pub fn accepts(automaton: &Automaton, input: &[Symbol]) -> bool {
    let mut current = epsilon_closure(automaton, HashSet::from([automaton.initial.clone()]));

    for symbol in input {
        let mut next = HashSet::new();
        for transition in automaton.transitions.iter().filter(|transition| {
            current.contains(&transition.from) && transition.input.as_ref() == Some(symbol)
        }) {
            next.extend(transition.to.iter().cloned());
        }
        current = epsilon_closure(automaton, next);
    }

    current.iter().any(|state| automaton.finals.contains(state))
}

fn epsilon_closure(automaton: &Automaton, seeds: HashSet<StateId>) -> HashSet<StateId> {
    let mut closure = seeds;
    let mut stack: Vec<StateId> = closure.iter().cloned().collect();

    while let Some(state) = stack.pop() {
        for transition in automaton
            .transitions
            .iter()
            .filter(|transition| transition.from == state && transition.input.is_none())
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

#[cfg(test)]
mod tests {
    use super::accepts;
    use crate::{Automaton, Symbol, parse};

    fn machine(source: &str) -> Automaton {
        parse("test.salmon", source).expect("simulation test input must parse")
    }

    fn symbol(value: &str) -> Symbol {
        Symbol(value.into())
    }

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
        let machine = machine("enfa x { initial:q0 final:q2 q0:eps=q1 q1:eps=[q0,q2] }");
        assert!(accepts(&machine, &[]));
    }

    #[test]
    fn unknown_symbols_are_rejected() {
        let machine = machine("nfa x { initial:q0 final:q1 q0:a=q1 }");
        assert!(!accepts(&machine, &[symbol("unknown")]));
    }
}
