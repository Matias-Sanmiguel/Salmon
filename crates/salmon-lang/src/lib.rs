pub mod diagnostic;
pub mod model;

pub use diagnostic::{Diagnostic, Severity, Span, render_diagnostic};
pub use model::{Automaton, AutomatonKind, StateId, Symbol, Transition};

#[allow(dead_code)]
mod ast;
#[allow(dead_code)]
mod lexer;

#[allow(dead_code)]
mod parser;

#[allow(dead_code)]
mod semantic;

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
