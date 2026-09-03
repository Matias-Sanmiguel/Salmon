use std::error::Error;
use std::fmt;

use salmon_lang::Symbol;

#[derive(Debug)]
pub(crate) struct InputError(String);

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for InputError {}

pub(crate) fn parse_input(input: &str, alphabet: &[Symbol]) -> Result<Vec<Symbol>, InputError> {
    if alphabet.iter().all(|symbol| symbol.0.chars().count() == 1)
        && !input.chars().any(|character| character.is_whitespace() || character == '"')
    {
        return Ok(input
            .chars()
            .map(|character| Symbol(character.to_string()))
            .collect());
    }

    let mut characters = input.chars().peekable();
    let mut symbols = Vec::new();

    while let Some(character) = characters.peek() {
        if character.is_whitespace() {
            characters.next();
            continue;
        }

        if *character == '"' {
            characters.next();
            let mut token = String::new();

            loop {
                match characters.next() {
                    Some('"') => break,
                    Some(character) => token.push(character),
                    None => return Err(InputError("unclosed quoted symbol".into())),
                }
            }

            if characters
                .peek()
                .is_some_and(|character| !character.is_whitespace())
            {
                return Err(InputError("unexpected quote in input".into()));
            }

            symbols.push(Symbol(token));
            continue;
        }

        let mut token = String::new();
        while let Some(character) = characters.peek() {
            if character.is_whitespace() {
                break;
            }
            if *character == '"' {
                return Err(InputError("unexpected quote in input".into()));
            }
            token.push(*character);
            characters.next();
        }
        symbols.push(Symbol(token));
    }

    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::parse_input;
    use salmon_lang::Symbol;

    fn symbols(values: &[&str]) -> Vec<Symbol> {
        values.iter().map(|value| Symbol((*value).into())).collect()
    }

    #[test]
    fn compact_input_splits_unicode_scalars_for_a_single_character_alphabet() {
        let alphabet = symbols(&["a", "ñ"]);
        assert_eq!(parse_input("añ", &alphabet).unwrap(), symbols(&["a", "ñ"]));
    }

    #[test]
    fn tokenized_input_supports_multiple_and_quoted_symbols() {
        let alphabet = symbols(&["open", "end value"]);
        assert_eq!(
            parse_input("open \"end value\"", &alphabet).unwrap(),
            symbols(&["open", "end value"]),
        );
    }

    #[test]
    fn mixed_alphabets_disable_compact_expansion() {
        let alphabet = symbols(&["a", "ab"]);
        assert_eq!(parse_input("ab", &alphabet).unwrap(), symbols(&["ab"]));
        assert_eq!(
            parse_input("a ab", &alphabet).unwrap(),
            symbols(&["a", "ab"])
        );
    }

    #[test]
    fn empty_input_is_the_empty_word() {
        assert!(parse_input("", &symbols(&["a"])).unwrap().is_empty());
    }

    #[test]
    fn rejects_unclosed_quotes() {
        let error = parse_input("open \"end value", &symbols(&["open"])).unwrap_err();
        assert_eq!(error.to_string(), "unclosed quoted symbol");
    }

    #[test]
    fn rejects_quotes_that_interrupt_an_unquoted_symbol() {
        let error = parse_input("open\"end\"", &symbols(&["open"])).unwrap_err();
        assert_eq!(error.to_string(), "unexpected quote in input");
    }

    #[test]
    fn rejects_text_immediately_after_a_quoted_symbol() {
        let error = parse_input("\"open\"end", &symbols(&["open"])).unwrap_err();
        assert_eq!(error.to_string(), "unexpected quote in input");
    }

    #[test]
    fn quoted_quit_is_a_regular_symbol() {
        assert_eq!(
            parse_input("\":quit\"", &symbols(&[":quit"])).unwrap(),
            symbols(&[":quit"])
        );
    }
}
