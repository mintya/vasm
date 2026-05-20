#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

impl Span {
    pub const fn new(line: u32, col: u32, len: u32) -> Self {
        Self { line, col, len }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }

    pub fn format(&self, file: &str) -> String {
        format!(
            "{}:{}:{}: {}: {}",
            file,
            self.span.line,
            self.span.col,
            self.severity.as_str(),
            self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_error_with_file_and_position() {
        let d = Diagnostic::error(Span::new(12, 7, 3), "unexpected token `foo`");
        assert_eq!(
            d.format("examples/hello.asm"),
            "examples/hello.asm:12:7: error: unexpected token `foo`"
        );
    }

    #[test]
    fn formats_warning() {
        let d = Diagnostic::warning(Span::new(1, 1, 1), "trailing whitespace");
        assert_eq!(d.format("a.asm"), "a.asm:1:1: warning: trailing whitespace");
    }
}
