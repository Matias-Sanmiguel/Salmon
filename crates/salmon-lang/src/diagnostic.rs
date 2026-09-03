#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
}

impl Severity {
    const fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub severity: Severity,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            severity: Severity::Error,
            span,
        }
    }
}

pub fn render_diagnostic(source_name: &str, source: &str, diagnostic: &Diagnostic) -> String {
    let prefix = &source[..diagnostic.span.start];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;

    format!(
        "{source_name}:{line}:{column}: {}[{}]: {}",
        diagnostic.severity.label(),
        diagnostic.code,
        diagnostic.message
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_source_position() {
        let diagnostic =
            Diagnostic::error("SEM004", "eps is not allowed in dfa", Span::new(10, 13));
        assert_eq!(
            render_diagnostic("machine.salmon", "dfa x {\n  eps\n}", &diagnostic),
            "machine.salmon:2:3: error[SEM004]: eps is not allowed in dfa"
        );
    }
}
