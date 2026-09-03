use crate::ast::{AstAutomaton, AstInput, AstItem, AstState, AstTransition, Spanned};
use crate::diagnostic::{Diagnostic, Span};
use crate::lexer::{Token, TokenKind};
use crate::model::AutomatonKind;

pub(crate) fn parse_tokens(tokens: &[Token]) -> Result<AstAutomaton, Vec<Diagnostic>> {
    if tokens.is_empty() {
        return Err(vec![Diagnostic::error(
            "SYN001",
            "expected an automaton declaration",
            Span::new(0, 0),
        )]);
    }

    let mut parser = Parser::new(tokens);
    let automaton = parser.parse_automaton();

    if parser.diagnostics.is_empty() {
        Ok(automaton.expect("a parser without diagnostics produces an automaton"))
    } else {
        Err(parser.diagnostics)
    }
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            position: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_automaton(&mut self) -> Option<AstAutomaton> {
        self.skip_newlines();
        let kind = self.parse_kind()?;
        let name = self.parse_identifier("an automaton name")?;
        self.expect("`{`", |kind| matches!(kind, TokenKind::LeftBrace))?;

        let mut items = Vec::new();
        let closing_span = loop {
            self.skip_newlines();
            match &self.current().kind {
                TokenKind::RightBrace => {
                    let span = self.current().span;
                    self.advance();
                    break span;
                }
                TokenKind::Eof => {
                    self.error("SYN002", "unclosed `{`", self.current().span);
                    return None;
                }
                _ => match self.parse_item() {
                    Some(item) => items.push(item),
                    None => self.recover_item(),
                },
            }
        };

        self.skip_newlines();
        if !matches!(self.current().kind, TokenKind::Eof) {
            self.error(
                "SYN001",
                "unexpected token after automaton declaration",
                self.current().span,
            );
        }

        let start = kind.span.start;
        Some(AstAutomaton {
            kind,
            name,
            items,
            span: Span::new(start, closing_span.end),
        })
    }

    fn parse_kind(&mut self) -> Option<Spanned<AutomatonKind>> {
        let token = self.current().clone();
        let value = match token.kind {
            TokenKind::Dfa => AutomatonKind::Dfa,
            TokenKind::Nfa => AutomatonKind::Nfa,
            TokenKind::Enfa => AutomatonKind::Enfa,
            _ => {
                self.error("SYN001", "expected `dfa`, `nfa`, or `enfa`", token.span);
                return None;
            }
        };
        self.advance();
        Some(Spanned {
            value,
            span: token.span,
        })
    }

    fn parse_item(&mut self) -> Option<AstItem> {
        match self.current().kind {
            TokenKind::Alphabet => self.parse_alphabet(),
            TokenKind::Initial => self.parse_initial(),
            TokenKind::Final => self.parse_final(),
            TokenKind::Identifier(_) => self.parse_state(),
            _ => {
                self.error(
                    "SYN001",
                    "expected a declaration or state row",
                    self.current().span,
                );
                None
            }
        }
    }

    fn parse_alphabet(&mut self) -> Option<AstItem> {
        let start = self.current().span.start;
        self.advance();
        self.expect("`:`", |kind| matches!(kind, TokenKind::Colon))?;

        let mut symbols = vec![self.parse_symbol("an alphabet symbol")?];
        self.skip_newlines();
        while matches!(self.current().kind, TokenKind::Comma) {
            self.advance();
            symbols.push(self.parse_symbol("an alphabet symbol after `,`")?);
            self.skip_newlines();
        }
        let end = symbols.last()?.span.end;
        Some(AstItem::Alphabet {
            symbols,
            span: Span::new(start, end),
        })
    }

    fn parse_initial(&mut self) -> Option<AstItem> {
        let start = self.current().span.start;
        self.advance();
        self.expect("`:`", |kind| matches!(kind, TokenKind::Colon))?;
        let state = self.parse_identifier("an initial state")?;
        Some(AstItem::Initial {
            span: Span::new(start, state.span.end),
            state,
        })
    }

    fn parse_final(&mut self) -> Option<AstItem> {
        let start = self.current().span.start;
        self.advance();
        self.expect("`:`", |kind| matches!(kind, TokenKind::Colon))?;

        let mut states = vec![self.parse_identifier("a final state")?];
        self.skip_newlines();
        while matches!(self.current().kind, TokenKind::Comma) {
            self.advance();
            states.push(self.parse_identifier("a final state after `,`")?);
            self.skip_newlines();
        }
        let end = states.last()?.span.end;
        Some(AstItem::Final {
            states,
            span: Span::new(start, end),
        })
    }

    fn parse_state(&mut self) -> Option<AstItem> {
        let name = self.parse_identifier("a state name")?;
        self.expect("`:`", |kind| matches!(kind, TokenKind::Colon))?;

        let mut transitions = vec![self.parse_transition()?];
        self.skip_newlines();
        while matches!(self.current().kind, TokenKind::Comma) {
            self.advance();
            transitions.push(self.parse_transition()?);
            self.skip_newlines();
        }
        let end = transitions.last()?.span.end;
        Some(AstItem::State(AstState {
            span: Span::new(name.span.start, end),
            name,
            transitions,
        }))
    }

    fn parse_transition(&mut self) -> Option<AstTransition> {
        let input = self.parse_input()?;
        self.expect("`=`", |kind| matches!(kind, TokenKind::Equals))?;
        let destinations = self.parse_destinations()?;
        let end = destinations.last()?.span.end;
        Some(AstTransition {
            span: Span::new(input.span.start, end),
            input,
            destinations,
        })
    }

    fn parse_input(&mut self) -> Option<Spanned<AstInput>> {
        self.skip_newlines();
        let token = self.current().clone();
        let value = match token.kind {
            TokenKind::Identifier(value) | TokenKind::String(value) => AstInput::Symbol(value),
            TokenKind::Epsilon => AstInput::Epsilon,
            _ => {
                self.error("SYN001", "expected a transition input", token.span);
                return None;
            }
        };
        self.advance();
        Some(Spanned {
            value,
            span: token.span,
        })
    }

    fn parse_destinations(&mut self) -> Option<Vec<Spanned<String>>> {
        if !matches!(self.current().kind, TokenKind::LeftBracket) {
            return Some(vec![self.parse_identifier("a destination state")?]);
        }

        self.advance();
        let mut destinations = vec![self.parse_identifier("a destination state")?];
        loop {
            self.skip_newlines();
            match self.current().kind {
                TokenKind::Comma => {
                    self.advance();
                    if self.destination_list_is_unclosed() {
                        self.error("SYN002", "unclosed `[`", self.current().span);
                        return None;
                    }
                    destinations.push(self.parse_identifier("a destination state after `,`")?);
                }
                TokenKind::RightBracket => {
                    self.advance();
                    return Some(destinations);
                }
                TokenKind::Eof | TokenKind::RightBrace => {
                    self.error("SYN002", "unclosed `[`", self.current().span);
                    return None;
                }
                _ if self.starts_item() => {
                    self.error("SYN002", "unclosed `[`", self.current().span);
                    return None;
                }
                _ => {
                    self.error(
                        "SYN001",
                        "expected `,` or `]` in destination list",
                        self.current().span,
                    );
                    return None;
                }
            }
        }
    }

    fn parse_symbol(&mut self, expected: &str) -> Option<Spanned<String>> {
        self.skip_newlines();
        let token = self.current().clone();
        let value = match token.kind {
            TokenKind::Identifier(value) | TokenKind::String(value) => value,
            _ => {
                self.error("SYN001", format!("expected {expected}"), token.span);
                return None;
            }
        };
        self.advance();
        Some(Spanned {
            value,
            span: token.span,
        })
    }

    fn parse_identifier(&mut self, expected: &str) -> Option<Spanned<String>> {
        self.skip_newlines();
        let token = self.current().clone();
        let TokenKind::Identifier(value) = token.kind else {
            self.error("SYN001", format!("expected {expected}"), token.span);
            return None;
        };
        self.advance();
        Some(Spanned {
            value,
            span: token.span,
        })
    }

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) {
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current().kind, TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect(
        &mut self,
        expected: &str,
        matches_expected: impl FnOnce(&TokenKind) -> bool,
    ) -> Option<Token> {
        self.skip_newlines();
        let token = self.current().clone();
        if matches_expected(&token.kind) {
            self.advance();
            Some(token)
        } else {
            self.error("SYN001", format!("expected {expected}"), token.span);
            None
        }
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message, span));
    }

    fn destination_list_is_unclosed(&mut self) -> bool {
        self.skip_newlines();
        matches!(self.current().kind, TokenKind::Eof | TokenKind::RightBrace) || self.starts_item()
    }

    fn starts_item(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Alphabet | TokenKind::Initial | TokenKind::Final
        ) || matches!(self.current().kind, TokenKind::Identifier(_))
            && self.next_non_newline_is_colon()
    }

    fn next_non_newline_is_colon(&self) -> bool {
        self.tokens
            .iter()
            .skip(self.position + 1)
            .find(|token| !matches!(token.kind, TokenKind::Newline))
            .is_some_and(|token| matches!(token.kind, TokenKind::Colon))
    }

    fn recover_item(&mut self) {
        while !matches!(
            self.current().kind,
            TokenKind::Newline | TokenKind::RightBrace | TokenKind::Eof
        ) && !self.starts_item()
        {
            self.advance();
        }
        if matches!(self.current().kind, TokenKind::Newline) {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstItem;
    use crate::lexer::lex;

    #[test]
    fn parses_compact_state_rows_and_multiple_destinations() {
        let source = "enfa example { initial: q0 final: q2 q0: a=[q0, q1], eps=q2 }";
        let (tokens, lex_errors) = lex(source);
        assert!(lex_errors.is_empty());
        let ast = parse_tokens(&tokens).unwrap();
        assert_eq!(ast.name.value, "example");
        let state = ast
            .items
            .iter()
            .find_map(|item| match item {
                AstItem::State(state) => Some(state),
                _ => None,
            })
            .unwrap();
        assert_eq!(state.transitions.len(), 2);
        assert_eq!(state.transitions[0].destinations.len(), 2);
    }

    #[test]
    fn rejects_a_second_automaton() {
        let (tokens, _) = lex("dfa a {} nfa b {}");
        let errors = parse_tokens(&tokens).unwrap_err();
        assert_eq!(errors[0].code, "SYN001");
    }

    #[test]
    fn recovers_at_a_newline_and_reports_multiple_errors() {
        let (tokens, _) = lex("dfa x {\ninitial q0\nfinal q1\n}");
        let errors = parse_tokens(&tokens).unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn reports_unclosed_braces_and_destination_lists() {
        for source in ["dfa x {", "nfa x { q0: a=[q1 }"] {
            let (tokens, _) = lex(source);
            let errors = parse_tokens(&tokens).unwrap_err();
            assert!(errors.iter().any(|error| error.code == "SYN002"));
        }
    }

    #[test]
    fn treats_newlines_as_trivia_inside_productions() {
        let source = "nfa x {\ninitial:\nq0\nq0:\na=q1\nq1: a=[q0,\nq1]\n}";
        let (tokens, lex_errors) = lex(source);
        assert!(lex_errors.is_empty());

        let ast = parse_tokens(&tokens).unwrap();
        let states = ast
            .items
            .iter()
            .filter_map(|item| match item {
                AstItem::State(state) => Some(state),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].transitions[0].destinations[0].value, "q1");
        assert_eq!(states[1].transitions[0].destinations.len(), 2);
    }

    #[test]
    fn reports_unclosed_destination_lists_at_item_boundaries() {
        for source in ["nfa x { q0: a=[q1\n}", "nfa x { q0: a=[q1 initial: q0 }"] {
            let (tokens, lex_errors) = lex(source);
            assert!(lex_errors.is_empty());

            let errors = parse_tokens(&tokens).unwrap_err();
            assert!(errors.iter().any(|error| error.code == "SYN002"));
        }
    }

    #[test]
    fn resumes_at_a_compact_declaration_after_an_unclosed_destination_list() {
        let (tokens, lex_errors) = lex("nfa x { q0: a=[q1 initial q0 }");
        assert!(lex_errors.is_empty());

        let errors = parse_tokens(&tokens).unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].code, "SYN002");
        assert_eq!(errors[1].code, "SYN001");
    }
}
