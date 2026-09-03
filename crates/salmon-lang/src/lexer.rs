use crate::diagnostic::{Diagnostic, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    Identifier(String),
    String(String),
    Dfa,
    Nfa,
    Enfa,
    Alphabet,
    Initial,
    Final,
    Epsilon,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Colon,
    Comma,
    Equals,
    Newline,
    Eof,
}

pub(crate) fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut index = 0;

    while index < source.len() {
        let rest = &source[index..];
        let character = rest.chars().next().expect("index is within source");

        match character {
            ' ' | '\t' | '\r' => index += character.len_utf8(),
            '\n' => {
                tokens.push(token(TokenKind::Newline, index, index + 1));
                index += 1;
            }
            '/' if rest.starts_with("//") => {
                index += 2;
                while index < source.len() && !source[index..].starts_with(['\r', '\n']) {
                    let comment_character = source[index..]
                        .chars()
                        .next()
                        .expect("index is within source");
                    index += comment_character.len_utf8();
                }
            }
            '"' => {
                let start = index;
                index += character.len_utf8();
                let contents_start = index;

                while index < source.len() {
                    let string_character = source[index..]
                        .chars()
                        .next()
                        .expect("index is within source");
                    if string_character == '"' || matches!(string_character, '\r' | '\n') {
                        break;
                    }
                    index += string_character.len_utf8();
                }

                if source[index..].starts_with('"') {
                    tokens.push(token(
                        TokenKind::String(source[contents_start..index].to_owned()),
                        start,
                        index + 1,
                    ));
                    index += 1;
                } else {
                    diagnostics.push(Diagnostic::error(
                        "SYN002",
                        "unclosed string",
                        Span::new(start, index),
                    ));
                }
            }
            '{' => {
                tokens.push(token(TokenKind::LeftBrace, index, index + 1));
                index += 1;
            }
            '}' => {
                tokens.push(token(TokenKind::RightBrace, index, index + 1));
                index += 1;
            }
            '[' => {
                tokens.push(token(TokenKind::LeftBracket, index, index + 1));
                index += 1;
            }
            ']' => {
                tokens.push(token(TokenKind::RightBracket, index, index + 1));
                index += 1;
            }
            ':' => {
                tokens.push(token(TokenKind::Colon, index, index + 1));
                index += 1;
            }
            ',' => {
                tokens.push(token(TokenKind::Comma, index, index + 1));
                index += 1;
            }
            '=' => {
                tokens.push(token(TokenKind::Equals, index, index + 1));
                index += 1;
            }
            _ if is_identifier_start(character) => {
                let start = index;
                index += character.len_utf8();
                while index < source.len() {
                    let identifier_character = source[index..]
                        .chars()
                        .next()
                        .expect("index is within source");
                    if !is_identifier_continue(identifier_character) {
                        break;
                    }
                    index += identifier_character.len_utf8();
                }
                let text = &source[start..index];
                tokens.push(token(keyword_or_identifier(text), start, index));
            }
            _ => {
                let end = index + character.len_utf8();
                diagnostics.push(Diagnostic::error(
                    "SYN001",
                    format!("unexpected character `{character}`"),
                    Span::new(index, end),
                ));
                index = end;
            }
        }
    }

    tokens.push(token(TokenKind::Eof, source.len(), source.len()));
    (tokens, diagnostics)
}

fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span::new(start, end),
    }
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    is_identifier_start(character) || character.is_numeric()
}

fn keyword_or_identifier(text: &str) -> TokenKind {
    match text {
        "dfa" => TokenKind::Dfa,
        "nfa" => TokenKind::Nfa,
        "enfa" => TokenKind::Enfa,
        "alphabet" => TokenKind::Alphabet,
        "initial" => TokenKind::Initial,
        "final" => TokenKind::Final,
        "eps" => TokenKind::Epsilon,
        _ => TokenKind::Identifier(text.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_keywords_symbols_strings_comments_and_punctuation() {
        let source = "dfa machine {\n// ignored\nq0: a=q1, \"end value\"=[q1]\n}";
        let (tokens, errors) = lex(source);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, TokenKind::Dfa);
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == TokenKind::String("end value".into()))
        );
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Newline));
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn reports_unexpected_characters_without_panicking() {
        let (_, errors) = lex("dfa x { q0: a=@ }");
        assert_eq!(errors[0].code, "SYN001");
        assert_eq!(errors[0].span, Span { start: 14, end: 15 });
    }

    #[test]
    fn reports_an_unclosed_string() {
        let (_, errors) = lex("dfa x { alphabet: \"end }");
        assert_eq!(errors[0].code, "SYN002");
    }
}
