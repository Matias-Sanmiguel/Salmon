use crate::diagnostic::Span;
use crate::model::AutomatonKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AstAutomaton {
    pub kind: Spanned<AutomatonKind>,
    pub name: Spanned<String>,
    pub items: Vec<AstItem>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AstItem {
    Alphabet {
        symbols: Vec<Spanned<String>>,
        span: Span,
    },
    Initial {
        state: Spanned<String>,
        span: Span,
    },
    Final {
        states: Vec<Spanned<String>>,
        span: Span,
    },
    State(AstState),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AstState {
    pub name: Spanned<String>,
    pub transitions: Vec<AstTransition>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AstTransition {
    pub input: Spanned<AstInput>,
    pub destinations: Vec<Spanned<String>>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AstInput {
    Symbol(String),
    Epsilon,
}
