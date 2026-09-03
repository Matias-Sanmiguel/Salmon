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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomatonKind {
    Dfa,
    Nfa,
    Enfa,
}

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
